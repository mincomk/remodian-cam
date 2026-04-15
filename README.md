# Remodian CAM

I've worked in a project called [Remodian](https://github.com/mincomk/remodian), which is a simple IR-Web bridge for Meridian speakers.

## The Problem

Previous Remodian was only capable to control the volume up and down, by one tick. So there was no way to know what the current volume level was, and it was impossible to set a specific volume level.

## My Solution
![meridian](/assets/meridian.jpg)
Unsurprisingly, the speakers have a volume display. Two green numbers showing current volume. So the next task is trivial: Computer Vision + Control System.

## Making ESP32 See the Volume
I noticed that the volume display is simple 5x7 LED grid. And the camera will always be in the same position. So I just divided the image to 5x7 regions, and averaged the pixel values in each region.

Since I could use thresholding to determine whether a region is lit or not, I went other approach: one-hot scoring. I created a template for each digit (0-9) that contains ideal state of each region (i.e. 255 for lit, 0 for unlit). Then I scored the current image against each template, and the one with the highest score is the recognized digit.

This approach avoids the need for thresholding, and is more robust to noise and lighting changes.

I've made a simple GUI tool called `calibration-ui` (`dev/calibration-ui`). This tool captures image from the camera, and allows me to configure perspective transform (i.e. warp crop) of the image. Then it produces standardized *crop string* (`1,1,1,1+1,1,1,1`) that can be used in the backend code to apply the same perspective transform.
![calibration-ui](/assets/calibration-ui.png)

## Control System
ESP32 IR blaster firmware accepts four commands: `volume up`, `volume down`, `volume rapid up start`, and `volume rapid up stop`. The first two commands will send one tick of volume up/down, while the last two commands will start/stop sending volume up command repeatedly with a short delay (RC5 IR timeframe).

The control logic is based on a simple bang-bang controller. It separates the difference between current volume and target volume into three zones: **Just Right** (i.e. deadband), **Very Near** (±4.0), **Near** (±8.0), and **Far** (±∞). Depending on which zone the difference falls into, the controller will send different commands to the IR blaster.
- If the difference is in the **Just Right** zone, do nothing.
- If the difference is in the **Very Near** zone, send one tick of volume up/down in slower pace.
- If the difference is in the **Near** zone, send one tick of volume up/down in faster pace.
- If the difference is in the **Far** zone, send rapid volume up/down command.

Also I added a manual control mode, which allows the user to send volume up/down commands directly from the web interface. Web interface also does the rapid command set on long press, and stop on release.

### Problem: ESP32-CAM is slow!
ESP32-CAM is a very cheap and convenient camera module, but it has a very slow processing speed. The image processing takes around 300ms, which is too slow for control system to be responsive.

But we have a helpful trait of the volume here: It changes consistently. So I used linear extrapolation to predict the current volume level based on the last two recognized volume levels and their timestamps. This way, we can get a more responsive control system, even with the slow image processing.

## Conclusion
![remodian](/assets/remodian.png)

LGTM! Let's enjoy the music.
