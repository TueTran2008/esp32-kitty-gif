use crate::consts::*;
use crate::error::Result;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::sys::*;
use log::info;

static VSYNC: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

pub(crate) fn lcd_init() -> Result<esp_lcd_panel_handle_t> {
    let display_width = DISPLAY_WIDTH as usize;
    let display_height = DISPLAY_HEIGHT as usize;

    info!("Display size: {}x{}", display_width, display_height);

    let hsync_idle_low = true;
    let vsync_idle_low = true;
    let de_idle_high = true;
    let pclk_active_neg = true;
    let pclk_idle_high = false;
    let mut panel_handle: esp_lcd_panel_handle_t = std::ptr::null_mut();
    let mut panel_config = esp_lcd_rgb_panel_config_t {
        clk_src: soc_periph_lcd_clk_src_t_LCD_CLK_SRC_PLL240M, //LCD_CLK_SRC_DEFAULT,
        timings: esp_lcd_rgb_timing_t {
            pclk_hz: (30 * 1000 * 1000) as u32,
            h_res: display_width as u32,
            v_res: display_height as u32,
            hsync_pulse_width: 2_u32,
            hsync_back_porch: 10_u32,
            hsync_front_porch: 8_u32,
            vsync_pulse_width: 2_u32,
            vsync_back_porch: 18_u32,
            vsync_front_porch: 50_u32,
            flags: Default::default(),
        },
        data_width: 16,
        bits_per_pixel: 16,
        num_fbs: 1,
        bounce_buffer_size_px: ((display_width * display_height) * 5) / 100,
        hsync_gpio_num: LCD_PIN_NUM_HSYNC,
        vsync_gpio_num: LCD_PIN_NUM_VSYNC,
        de_gpio_num: LCD_PIN_NUM_DE,
        pclk_gpio_num: LCD_PIN_NUM_PCLK,
        disp_gpio_num: LCD_PIN_NUM_DISP,
        data_gpio_nums: [
            LCD_PIN_NUM_DATA0,
            LCD_PIN_NUM_DATA1,
            LCD_PIN_NUM_DATA2,
            LCD_PIN_NUM_DATA3,
            LCD_PIN_NUM_DATA4,
            LCD_PIN_NUM_DATA5,
            LCD_PIN_NUM_DATA6,
            LCD_PIN_NUM_DATA7,
            LCD_PIN_NUM_DATA8,
            LCD_PIN_NUM_DATA9,
            LCD_PIN_NUM_DATA10,
            LCD_PIN_NUM_DATA11,
            LCD_PIN_NUM_DATA12,
            LCD_PIN_NUM_DATA13,
            LCD_PIN_NUM_DATA14,
            LCD_PIN_NUM_DATA15,
        ],
        sram_trans_align: 4,
        psram_trans_align: 64,
        flags: Default::default(),
    };
    panel_config.flags.set_fb_in_psram(1);
    panel_config
        .timings
        .flags
        .set_hsync_idle_low(hsync_idle_low as _);
    panel_config
        .timings
        .flags
        .set_vsync_idle_low(vsync_idle_low as _);
    panel_config
        .timings
        .flags
        .set_de_idle_high(de_idle_high as _);
    panel_config
        .timings
        .flags
        .set_pclk_active_neg(pclk_active_neg as _);
    panel_config
        .timings
        .flags
        .set_pclk_idle_high(pclk_idle_high as _);
    unsafe {
        assert_eq!(
            esp_lcd_new_rgb_panel(&panel_config, &mut panel_handle),
            ESP_OK
        );
        assert_eq!(esp_lcd_panel_init(panel_handle), ESP_OK);
        assert_eq!(
            esp_lcd_rgb_panel_register_event_callbacks(
                panel_handle,
                &esp_lcd_rgb_panel_event_callbacks_t {
                    on_vsync: Some(vsync_callback),
                    on_bounce_empty: None,
                    ..Default::default()
                },
                core::ptr::null_mut()
            ),
            ESP_OK
        );
    }
    Ok(panel_handle)
}

unsafe extern "C" fn vsync_callback(
    _panel: esp_idf_svc::hal::sys::esp_lcd_panel_handle_t,
    _edata: *const esp_idf_svc::hal::sys::esp_lcd_rgb_panel_event_data_t,
    _user_ctx: *mut core::ffi::c_void,
) -> bool {
    VSYNC.store(true, core::sync::atomic::Ordering::SeqCst);
    false
}
