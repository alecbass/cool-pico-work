pub(super) const VL51L1X_DEFAULT_CONFIGURATION: &[u8] = &[
    0x00, // Register padding
    0x2D, // Register
    0x00, // 0x2d : set bit 2 and 5 to 1 for fast plus mode (1MHz I2C), else don't touch */
    0x00, // 0x2e : bit 0 if I2C pulled up at 1.8V, else set bit 0 to 1 (pull up at AVDD) */
    0x00, // 0x2f : bit 0 if GPIO pulled up at 1.8V, else set bit 0 to 1 (pull up at AVDD) */
    0x01, // 0x30 : set bit 4 to 0 for active high interrupt and 1 for active low (bits 3:0 must be 0x1), use SetInterruptPolarity() */
    0x02, // 0x31 : bit 1 = interrupt depending on the polarity, use CheckForDataReady() */
    0x00, // 0x32 : not user-modifiable (NUM)*/
    0x02, // 0x33 : NUM */
    0x08, // 0x34 : NUM */
    0x00, // 0x35 : NUM */
    0x08, // 0x36 : NUM */
    0x10, // 0x37 : NUM */
    0x01, // 0x38 : NUM */
    0x01, // 0x39 : NUM */
    0x00, // 0x3a : NUM */
    0x00, // 0x3b : NUM */
    0x00, // 0x3c : NUM */
    0x00, // 0x3d : NUM */
    0xff, // 0x3e : NUM */
    0x00, // 0x3f : NUM */
    0x0F, // 0x40 : NUM */
    0x00, // 0x41 : NUM */
    0x00, // 0x42 : NUM */
    0x00, // 0x43 : NUM */
    0x00, // 0x44 : NUM */
    0x00, // 0x45 : NUM */
    0x20, // 0x46 : interrupt configuration 0->level low detection, 1-> level high, 2-> Out of window, 3->In window, 0x20-> New sample ready , TBC */
    0x0b, // 0x47 : NUM */
    0x00, // 0x48 : NUM */
    0x00, // 0x49 : NUM */
    0x02, // 0x4a : NUM */
    0x0a, // 0x4b : NUM */
    0x21, // 0x4c : NUM */
    0x00, // 0x4d : NUM */
    0x00, // 0x4e : NUM */
    0x05, // 0x4f : NUM */
    0x00, // 0x50 : NUM */
    0x00, // 0x51 : NUM */
    0x00, // 0x52 : NUM */
    0x00, // 0x53 : NUM */
    0xc8, // 0x54 : NUM */
    0x00, // 0x55 : NUM */
    0x00, // 0x56 : NUM */
    0x38, // 0x57 : NUM */
    0xff, // 0x58 : NUM */
    0x01, // 0x59 : NUM */
    0x00, // 0x5a : NUM */
    0x08, // 0x5b : NUM */
    0x00, // 0x5c : NUM */
    0x00, // 0x5d : NUM */
    0x01, // 0x5e : NUM */
    0xdb, // 0x5f : NUM */
    0x0f, // 0x60 : NUM */
    0x01, // 0x61 : NUM */
    0xf1, // 0x62 : NUM */
    0x0d, // 0x63 : NUM */
    0x01, // 0x64 : Sigma threshold MSB (mm in 14.2 format for MSB+LSB), use SetSigmaThreshold(), default value 90 mm  */
    0x68, // 0x65 : Sigma threshold LSB */
    0x00, // 0x66 : Min count Rate MSB (MCPS in 9.7 format for MSB+LSB), use SetSignalThreshold() */
    0x80, // 0x67 : Min count Rate LSB */
    0x08, // 0x68 : NUM */
    0xb8, // 0x69 : NUM */
    0x00, // 0x6a : NUM */
    0x00, // 0x6b : NUM */
    0x00, // 0x6c : Intermeasurement period MSB, 32 bits register, use SetIntermeasurementInMs() */
    0x00, // 0x6d : Intermeasurement period */
    0x0f, // 0x6e : Intermeasurement period */
    0x89, // 0x6f : Intermeasurement period LSB */
    0x00, // 0x70 : NUM */
    0x00, // 0x71 : NUM */
    0x00, // 0x72 : distance threshold high MSB (in mm, MSB+LSB), use SetD:tanceThreshold() */
    0x00, // 0x73 : distance threshold high LSB */
    0x00, // 0x74 : distance threshold low MSB ( in mm, MSB+LSB), use SetD:tanceThreshold() */
    0x00, // 0x75 : distance threshold low LSB */
    0x00, // 0x76 : NUM */
    0x01, // 0x77 : NUM */
    0x0f, // 0x78 : NUM */
    0x0d, // 0x79 : NUM */
    0x0e, // 0x7a : NUM */
    0x0e, // 0x7b : NUM */
    0x00, // 0x7c : NUM */
    0x00, // 0x7d : NUM */
    0x02, // 0x7e : NUM */
    0xc7, // 0x7f : ROI center, use SetROI() */
    0xff, // 0x80 : XY ROI (X=Width, Y=Height), use SetROI() */
    0x9B, // 0x81 : NUM */
    0x00, // 0x82 : NUM */
    0x00, // 0x83 : NUM */
    0x00, // 0x84 : NUM */
    0x01, // 0x85 : NUM */
    0x01, // 0x86 : clear interrupt, use ClearInterrupt() */
    0x40, // 0x87 : start ranging, use StartRanging() or StopRanging(), If you want an automatic start after VL53L1X_init() call, put 0x40 in location 0x87 */
];

pub(super) const _DEFAULT_MODEL_ID: u16 = 0xEACC;

// Device address
pub(super) const BASE_ADDR: u8 = 0x29;
