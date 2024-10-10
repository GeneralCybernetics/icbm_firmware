# Main

### Modes & Filters
- Three modes
    - Mode 0: Sleep Mode, no measurements made
    - Mode 1: Stream Mode, measurements made & reported twice per second
    - Mode 2: Polling Mode, only reported when requested
- The sensor will power up in mode 1/2 whichever one was used before the power cycle
- There is a tradeoff between the digital filter accuracy and time to measurement, filter values can also be set manually

### Zero point settings
- used to calibrate the baseline of the sensor
- the possible methods/commands:
    - in a known concentration
    - in pure nitrogen (0 ppm)
    - in fresh air (assumed 400 ppm)
    - Zero Point Adjustment (both known concentration and sensors reported concentration)
    - Auto Zero:
        - Assumes that the sensor will be exposed to fresh air (lowest CO2 level) periodically (the period can be programmed)
        - Automatically adjusts the zero point based on the lowest readings over time
    
### Pressure and Concentration
- Just refer the table

## Functions that can be added later if needed
- Setting the value of Digital Filter
- Returning the value of Digital Filter
- Zero point using Fresh Air (G command)
- Expanded measurement datatypes
- Any co2 auto zero related commands (P commands)
- Set fresh air co2 zero point
- Zero point setting using nitrogen
- Manual zero point setting
- Setting Auto zero time intervals
- Auto zero config returner
- Switching Auto-Zero off (disabled by default)
- Scale factor returner (it is 100)