#include "DEV_Config.h"
#include "EPD_1in54b.h"
#include "EPD_Test.h"
#include "GUI_Paint.h"
#include "eink/lib/Fonts/fonts.h"
#include <hardware/gpio.h>
#include <pico/stdlib.h>

#include "atmospheric_sensor.h"
#include "eink.h"

Eink* initEink() {
    printf("Starting EPD.......\r\n");

    DEV_Delay_ms(500);

    // int testResult = EPD_1in54b_test();
    // printf("Test result: %d\n", testResult);

    UWORD Imagesize =
        ((EPD_1IN54B_WIDTH % 8 == 0) ? (EPD_1IN54B_WIDTH / 8) : (EPD_1IN54B_WIDTH / 8 + 1)) * EPD_1IN54B_HEIGHT;
    UBYTE* BlackImage = (UBYTE*)malloc(Imagesize);
    UBYTE* RedImage = (UBYTE*)malloc(Imagesize);

    if (BlackImage == NULL) {
        printf("Failed to apply for black memory...\r\n");
        return NULL;
    }

    if (RedImage == NULL) {
        printf("Failed to apply for red memory...\r\n");
        free(BlackImage);
        return NULL;
    }

    printf("EPD_1in54b_test Demo\r\n");
    if (DEV_Module_Init() != 0) {
        free(BlackImage);
        free(RedImage);
        return NULL;
    }

    printf("e-Paper Init and Clear...\r\n");
    EPD_1IN54B_Init();
    EPD_1IN54B_Clear();
    printf("e-Paper inited and cleared...\r\n");
    DEV_Delay_ms(500);

    Paint_NewImage(BlackImage, EPD_1IN54B_WIDTH, EPD_1IN54B_HEIGHT, 270, WHITE);
    Paint_NewImage(RedImage, EPD_1IN54B_WIDTH, EPD_1IN54B_HEIGHT, 270, WHITE);

    // printf("show window BMP-----------------\r\n");
    // printf("read black bmp\r\n");
    // Paint_SelectImage(BlackImage);
    // // GUI_ReadBmp("./pic/100x100.bmp", 50, 50);
    //
    // Paint_SelectImage(RedImage);
    // Paint_Clear(WHITE);
    //
    // EPD_1IN54B_Display(BlackImage, RedImage);
    // DEV_Delay_ms(2000);
    //
    // printf("show bmp------------------------\r\n");
    // printf("read black bmp\r\n");
    // Paint_SelectImage(BlackImage);
    // // GUI_ReadBmp("./pic/1in54b-b.bmp", 0, 0);
    // printf("read red bmp\r\n");
    // Paint_SelectImage(RedImage);
    // // GUI_ReadBmp("./pic/1in54b-r.bmp", 0, 0);
    //
    // EPD_1IN54B_Display(BlackImage, RedImage);
    // DEV_Delay_ms(2000);

    // printf("Drawing------------------------\r\n");
    // Paint_SelectImage(BlackImage);
    // Paint_Clear(WHITE);
    // Paint_DrawPoint(5, 10, BLACK, DOT_PIXEL_1X1, DOT_STYLE_DFT);
    // Paint_DrawPoint(5, 25, BLACK, DOT_PIXEL_2X2, DOT_STYLE_DFT);
    // Paint_DrawLine(20, 10, 70, 60, BLACK, DOT_PIXEL_1X1, LINE_STYLE_SOLID);
    // Paint_DrawLine(70, 10, 20, 60, BLACK, DOT_PIXEL_1X1, LINE_STYLE_SOLID);
    // Paint_DrawRectangle(20, 10, 70, 60, BLACK, DOT_PIXEL_1X1, DRAW_FILL_EMPTY);
    // Paint_DrawCircle(170, 85, 20, BLACK, DOT_PIXEL_1X1, DRAW_FILL_FULL);
    // Paint_DrawString_EN(5, 70, "hello world", &Font16, WHITE, BLACK);
    // Paint_DrawString_CN(5, 160, "Î¢Ñ©µç×Ó", &Font24CN, WHITE, BLACK);
    //
    // Paint_SelectImage(RedImage);
    // Paint_Clear(WHITE);
    // Paint_DrawPoint(5, 40, BLACK, DOT_PIXEL_3X3, DOT_STYLE_DFT);
    // Paint_DrawPoint(5, 55, BLACK, DOT_PIXEL_4X4, DOT_STYLE_DFT);
    // Paint_DrawLine(170, 15, 170, 55, BLACK, DOT_PIXEL_1X1, LINE_STYLE_DOTTED);
    // Paint_DrawLine(150, 35, 190, 35, BLACK, DOT_PIXEL_1X1, LINE_STYLE_DOTTED);
    // Paint_DrawRectangle(85, 10, 130, 60, BLACK, DOT_PIXEL_1X1, DRAW_FILL_FULL);
    // Paint_DrawCircle(170, 35, 20, BLACK, DOT_PIXEL_1X1, DRAW_FILL_EMPTY);
    // Paint_DrawString_EN(5, 90, "waveshare", &Font20, BLACK, WHITE);
    // Paint_DrawNum(5, 120, 123456789, &Font20, BLACK, WHITE);
    // Paint_DrawString_CN(5, 135, "ÄãºÃabc", &Font12CN, BLACK, WHITE);

    // EPD_1IN54B_Display(BlackImage, RedImage);
    DEV_Delay_ms(2000);
    printf("Displayed image\n");

    Eink* eink = malloc(sizeof(Eink));
    eink->BlackImage = BlackImage;
    eink->RedImage = RedImage;

    return eink;
}

void printTemperature(Eink* eink, TemperatureReading reading) {
    char temperatureStringBuffer[6];
    sprintf(temperatureStringBuffer, "%.2fc", reading.temperature);

    bool isHot = reading.temperature > 20.0;
    Paint_Clear(WHITE);
    Paint_SelectImage(isHot ? eink->RedImage : eink->BlackImage);
    Paint_Clear(WHITE);
    Paint_DrawString_EN(0, 40, "JARTIS TEMPERATURE", &Font16, WHITE, BLACK);
    Paint_DrawString_EN(40, 100, temperatureStringBuffer, &Font16, WHITE, BLACK);

    if (isHot) {
        Paint_DrawCircle(20, 180, 20, BLACK, DOT_PIXEL_1X1, DRAW_FILL_FULL);
        Paint_DrawCircle(180, 180, 20, BLACK, DOT_PIXEL_1X1, DRAW_FILL_FULL);
        Paint_DrawString_EN(40, 160, "IT'S HOT!!", &Font16, WHITE, RED);
    }

    printf("Displaying...\n");
    EPD_1IN54B_Display(eink->BlackImage, eink->RedImage);
}
