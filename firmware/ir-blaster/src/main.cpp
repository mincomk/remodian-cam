#include <Arduino.h>
#include <WiFi.h>
#include <WebServer.h>
#include <IRremoteESP8266.h>
#include <IRsend.h>

// ── WiFi credentials ──────────────────────────────────────────────────────────
#define WIFI_SSID "wlo1"
#define WIFI_PASS "haskell1234"

// ── IR transmitter ────────────────────────────────────────────────────────────
#define IR_TX_PIN 12  // GPIO12 — SD_DATA2, free when SD card unused

IRsend ir(IR_TX_PIN);

// ── IR command queue and synchronization ──────────────────────────────────────
struct IRCommand {
    uint8_t addr;
    uint8_t cmd;
    bool    toggle;
};

static SemaphoreHandle_t irMutex        = nullptr;
static QueueHandle_t     irOneShotQueue = nullptr;

static bool    irContinuous = false;
static uint8_t irContAddr   = 0;
static uint8_t irContCmd    = 0;
static bool    irContToggle = false;

WebServer server(80);

// ── Route: POST /ir ───────────────────────────────────────────────────────────
// Params: address (0-31), command (0-127), toggle (0|1), signal (1|2|3)
// signal 1 = single fire (enqueued), 2 = start continuous, 3 = stop
void handleIR() {
    if (!server.hasArg("address") || !server.hasArg("command") ||
        !server.hasArg("toggle")  || !server.hasArg("signal")) {
        server.send(400, "text/plain", "Missing params: address,command,toggle,signal");
        return;
    }

    uint8_t address = (uint8_t)server.arg("address").toInt();
    uint8_t command = (uint8_t)server.arg("command").toInt();
    bool    toggle  = server.arg("toggle").toInt() != 0;
    uint8_t signal  = (uint8_t)server.arg("signal").toInt();

    Serial.printf("[IR] addr=%u cmd=%u toggle=%u signal=%u\n",
                  address, command, (uint8_t)toggle, signal);

    switch (signal) {
        case 1: {
            IRCommand cmd = {address, command, toggle};
            if (xQueueSend(irOneShotQueue, &cmd, 0) == pdTRUE) {
                Serial.println("[IR] single fire enqueued");
            } else {
                Serial.println("[IR] single fire dropped (queue full)");
            }
            server.send(200, "text/plain", "ok");
            break;
        }
        case 2:
            if (xSemaphoreTake(irMutex, pdMS_TO_TICKS(10)) == pdTRUE) {
                irContAddr   = address;
                irContCmd    = command;
                irContToggle = toggle;
                irContinuous = true;
                xSemaphoreGive(irMutex);
                Serial.println("[IR] continuous start");
            } else {
                Serial.println("[IR] continuous start failed (mutex timeout)");
            }
            server.send(200, "text/plain", "ok");
            break;
        case 3:
            if (xSemaphoreTake(irMutex, pdMS_TO_TICKS(10)) == pdTRUE) {
                irContinuous = false;
                xSemaphoreGive(irMutex);
                Serial.println("[IR] continuous stop");
            } else {
                Serial.println("[IR] continuous stop failed (mutex timeout)");
            }
            server.send(200, "text/plain", "ok");
            break;
        default:
            server.send(400, "text/plain", "Invalid signal value (use 1, 2, or 3)");
    }
}

// ── Route: GET /ir/test — blink IR pin 5× at ~10 Hz to verify hardware ───────
// Visible as flicker on a phone camera (IR appears purple/white).
void handleIRTest() {
    Serial.printf("[IR] hardware test on GPIO%d\n", IR_TX_PIN);

    // Pause IR continuous mode and flush one-shot queue during test
    if (xSemaphoreTake(irMutex, pdMS_TO_TICKS(10)) == pdTRUE) {
        irContinuous = false;
        xQueueReset(irOneShotQueue);
        xSemaphoreGive(irMutex);
    }

    for (int i = 0; i < 5; i++) {
        digitalWrite(IR_TX_PIN, HIGH);
        vTaskDelay(pdMS_TO_TICKS(50));
        digitalWrite(IR_TX_PIN, LOW);
        vTaskDelay(pdMS_TO_TICKS(50));
    }
    server.send(200, "text/plain", "IR pin blink done — check phone camera for purple flashes on GPIO" + String(IR_TX_PIN));
}

// ── Route: GET / (simple info page) ──────────────────────────────────────────
void handleRoot() {
    server.send(200, "text/html",
        "<h1>ESP32 IR Blaster</h1>"
        "<p>IR control device — use HTTP API to send IR commands</p>");
}

// ── IR task — Core 0, handles all IR transmission ─────────────────────────────
void irTask(void *pvParameters) {
    TickType_t xLastWakeTime = xTaskGetTickCount();

    for (;;) {
        // Check for one-shot IR command
        IRCommand oneShot;
        if (xQueueReceive(irOneShotQueue, &oneShot, 0) == pdTRUE) {
            ir.sendRC5(ir.encodeRC5X(oneShot.addr, oneShot.cmd, oneShot.toggle));
        }

        // Check and handle continuous IR mode
        bool doSend = false;
        uint8_t addr, cmd;
        bool tog;

        if (xSemaphoreTake(irMutex, pdMS_TO_TICKS(5)) == pdTRUE) {
            if (irContinuous) {
                doSend = true;
                addr = irContAddr;
                cmd = irContCmd;
                tog = irContToggle;
            }
            xSemaphoreGive(irMutex);
        }

        if (doSend) {
            ir.sendRC5(ir.encodeRC5X(addr, cmd, tog));
        }

        // Delay until next 117ms period
        xTaskDelayUntil(&xLastWakeTime, pdMS_TO_TICKS(117));
    }
}

// ── HTTP task — Core 1, handles all HTTP serving ────────────────────────────
void httpTask(void *pvParameters) {
    for (;;) {
        server.handleClient();
        vTaskDelay(pdMS_TO_TICKS(1));  // Yield to prevent WDT starvation
    }
}

// ── Setup ─────────────────────────────────────────────────────────────────────
void setup() {
    Serial.begin(115200);

    WiFi.begin(WIFI_SSID, WIFI_PASS);
    Serial.print("Connecting to WiFi");
    while (WiFi.status() != WL_CONNECTED) {
        delay(500);
        Serial.print('.');
    }
    ir.begin();
    server.on("/", HTTP_GET, handleRoot);
    server.on("/ir", HTTP_POST, handleIR);
    server.on("/ir/test", HTTP_GET, handleIRTest);
    server.begin();
    Serial.printf("\nHTTP server started. Board IP: http://%s\n", WiFi.localIP().toString().c_str());

    // Create FreeRTOS synchronization primitives
    irMutex = xSemaphoreCreateMutex();
    irOneShotQueue = xQueueCreate(2, sizeof(IRCommand));

    if (irMutex == nullptr || irOneShotQueue == nullptr) {
        Serial.println("ERROR: Failed to create IR synchronization primitives!");
        return;
    }

    // Launch IR task on Core 0
    xTaskCreatePinnedToCore(
        irTask,
        "irTask",
        4096,
        nullptr,
        2,
        nullptr,
        0
    );

    // Launch HTTP task on Core 1
    xTaskCreatePinnedToCore(
        httpTask,
        "httpTask",
        8192,
        nullptr,
        1,
        nullptr,
        1
    );

    Serial.println("FreeRTOS tasks created: irTask on Core 0, httpTask on Core 1");
}

// ── Loop ──────────────────────────────────────────────────────────────────────
// Tasks are now handled by irTask (Core 0) and httpTask (Core 1).
// This loop task is suspended and yields to prevent watchdog timer starvation.
void loop() {
    vTaskDelay(portMAX_DELAY);
}
