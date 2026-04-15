import { useState, useEffect } from "react";
import {
    volumeDown,
    volumeUp,
    rapidUp,
    rapidDown,
    rapidStop,
    setVolume,
    setManual,
    openVolumeEvents,
    type VolumeState,
} from "../api";
import { Button } from "./ui/button";
import { SliderChart } from "./slider-chart";

const QUICK_SET_VOLUMES = [35, 40, 45, 50, 60, 65];
const MAX_HISTORY = 60;

type HistoryPoint = {
    time: number;
    desired: number;
    expected: number | null;
    current: number | null;
};

export function VolumeControl() {
    const [volumeState, setVolumeState] = useState<VolumeState | null>(null);
    const [sliderValue, setSliderValue] = useState(50);
    const [history, setHistory] = useState<HistoryPoint[]>([]);

    useEffect(() => {
        const eventSource = openVolumeEvents();

        const handleMessage = (event: MessageEvent) => {
            try {
                const data = JSON.parse(event.data) as VolumeState;
                setVolumeState(prev => {
                    // Initialize slider to desired on first connection only
                    if (!prev) setSliderValue(data.desired);
                    return data;
                });

                // Add to history
                setHistory(prev => {
                    const next = [
                        ...prev,
                        {
                            time: Date.now(),
                            desired: data.desired,
                            expected: data.expected,
                            current: data.current,
                        },
                    ];
                    return next.length > MAX_HISTORY
                        ? next.slice(-MAX_HISTORY)
                        : next;
                });
            } catch (error) {
                console.error("Failed to parse volume state:", error);
            }
        };

        const handleError = () => {
            console.error("SSE connection error");
            eventSource.close();
        };

        eventSource.addEventListener("message", handleMessage);
        eventSource.addEventListener("error", handleError);

        return () => {
            eventSource.removeEventListener("message", handleMessage);
            eventSource.removeEventListener("error", handleError);
            eventSource.close();
        };
    }, []);

    const isDisabled = !volumeState || volumeState.off;

    const handleQuickSet = (volume: number) => {
        setSliderValue(volume);
        setVolume(volume).catch((error) => {
            console.error("Failed to set volume:", error);
        });
    };

    const handleSetManual = () => {
        setManual().catch((error) => {
            console.error("Failed to set manual mode:", error);
        });
    };

    return (
        <div className="flex flex-col gap-4">
            {/* Mode badge and controls */}
            <div className="flex items-center justify-between px-2">
                <div className="flex items-center gap-2">
                    <span className="text-sm font-medium">Mode:</span>
                    <span
                        className={`text-sm font-semibold px-3 py-1 rounded-full ${
                            volumeState?.automatic
                                ? "bg-blue-200 text-blue-900"
                                : "bg-gray-200 text-gray-900"
                        }`}
                    >
                        {volumeState?.automatic ? "Automatic" : "Manual"}
                    </span>
                </div>
                <div className="flex flex-row gap-3">
                    {volumeState?.automatic && (
                        <button
                            onClick={handleSetManual}
                            className="text-sm bg-gray-300 hover:bg-gray-400 text-gray-900 font-semibold py-1 px-3 rounded-lg transition-colors"
                        >
                            Set Manual
                        </button>
                    )}
                    <Button
                        singleFunction={volumeDown}
                        rapidStart={rapidDown}
                        rapidEnd={rapidStop}
                    >
                        V-
                    </Button>
                    <Button
                        singleFunction={volumeUp}
                        rapidStart={rapidUp}
                        rapidEnd={rapidStop}
                    >
                        V+
                    </Button>
                </div>
            </div>

            {/* Volume status text box */}
            {volumeState && (
                <div className="bg-gray-800 border border-gray-700 rounded-lg px-4 py-3 mx-2">
                    <div className="text-sm font-mono text-white">
                        <div>
                            Desired: <span className="font-semibold">{volumeState.desired}</span>
                        </div>
                        <div>
                            Current: <span className="font-semibold">{volumeState.current}</span>
                        </div>
                    </div>
                </div>
            )}

            {/* Slider + Chart layout */}
            {isDisabled ? (
                <div className="text-sm text-gray-400 text-center py-4">
                    Volume unknown (disabled)
                </div>
            ) : (
                <SliderChart
                    value={sliderValue}
                    history={history}
                    disabled={isDisabled}
                    onChange={setSliderValue}
                    onCommit={(v) => setVolume(v).catch(console.error)}
                />
            )}

            {/* Quick set buttons */}
            <div className="flex flex-wrap gap-2 justify-center">
                {QUICK_SET_VOLUMES.map((volume) => (
                    <button
                        key={volume}
                        onClick={() => handleQuickSet(volume)}
                        disabled={isDisabled}
                        className="bg-gray-300 py-2 px-4 rounded-lg border-2 border-gray-200 hover:border-gray-300 hover:bg-gray-400 disabled:bg-gray-300 disabled:border-gray-200 disabled:cursor-not-allowed disabled:opacity-50"
                    >
                        {volume}
                    </button>
                ))}
            </div>
        </div>
    );
}
