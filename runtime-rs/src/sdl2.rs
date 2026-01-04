//! SDL2 Graphics Library Bindings for Tarqeem (مكتبة الرسوميات)
//!
//! This module provides FFI-compatible bindings for SDL2 graphics functionality.
//! All functions use the C calling convention for compatibility with compiled Tarqeem programs.
//!
//! # Arabic Naming Convention
//! Following Tarqeem's philosophy of authentic Arabic programming:
//! - نافذة (nāfidha) = window
//! - رسام (rassām) = renderer
//! - لون (lawn) = color
//! - حدث (hadath) = event
//! - مستطيل (mustatīl) = rectangle
//! - دائرة (dā'ira) = circle
//! - نقطة (nuqta) = point
//! - صورة (sūra) = image/texture
//!
//! # Usage
//! This module requires the `graphics` feature to be enabled:
//! ```toml
//! [dependencies]
//! tarqeem-runtime = { version = "1.0", features = ["graphics"] }
//! ```
//!
//! # Thread Safety Note
//! SDL2 must be used from the main thread. All SDL2 state is stored in
//! thread-local storage to ensure single-threaded access.

use crate::types::TrqString;
use sdl2::event::Event;
use sdl2::mouse::MouseButton;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, TextureCreator};
use sdl2::video::{Window, WindowContext};
use sdl2::EventPump;
use sdl2::Sdl;
use std::cell::RefCell;
use std::collections::HashMap;

// ============================================================================
// Thread-Local State Management
// SDL2 is not thread-safe, so we use thread-local storage
// ============================================================================

thread_local! {
    /// Global SDL2 context - initialized once per program
    static SDL_CONTEXT: RefCell<Option<Sdl>> = const { RefCell::new(None) };

    /// Window handle counter
    static NEXT_WINDOW_ID: RefCell<i64> = const { RefCell::new(1) };

    /// Active windows storage (not const because HashMap::new() isn't const)
    static WINDOWS: RefCell<HashMap<i64, WindowData>> = RefCell::new(HashMap::new());

    /// Active event pump (one per SDL context)
    static EVENT_PUMP: RefCell<Option<EventPump>> = const { RefCell::new(None) };

    /// Current event data for polling
    static CURRENT_EVENT: RefCell<TrqEvent> = const { RefCell::new(TrqEvent::default_const()) };
}

/// Storage for window data including canvas
struct WindowData {
    canvas: Canvas<Window>,
    // TextureCreator is stored here for texture loading
    _texture_creator: TextureCreator<WindowContext>,
}

// ============================================================================
// FFI-Compatible Structures
// ============================================================================

/// Event types enumeration
/// Arabic: أنواع الأحداث
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrqEventType {
    /// No event / unknown
    None = 0,
    /// Window close requested - إغلاق
    Quit = 1,
    /// Key pressed - ضغط_مفتاح
    KeyDown = 2,
    /// Key released - رفع_مفتاح
    KeyUp = 3,
    /// Mouse moved - حركة_فأرة
    MouseMotion = 4,
    /// Mouse button pressed - ضغط_فأرة
    MouseButtonDown = 5,
    /// Mouse button released - رفع_فأرة
    MouseButtonUp = 6,
    /// Window resized - تغيير_حجم
    WindowResized = 7,
    /// Mouse wheel scrolled - تمرير_فأرة
    MouseWheel = 8,
}

/// Event structure for Tarqeem
/// Arabic: بنية الحدث
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TrqEvent {
    /// Event type - نوع الحدث
    pub event_type: TrqEventType,
    /// Key code (for keyboard events) - رمز المفتاح
    pub key_code: i64,
    /// Mouse X position - موضع_س
    pub mouse_x: i64,
    /// Mouse Y position - موضع_ص
    pub mouse_y: i64,
    /// Mouse button (1=left, 2=middle, 3=right) - زر الفأرة
    pub mouse_button: i64,
    /// Window width (for resize events) - عرض النافذة
    pub window_width: i64,
    /// Window height (for resize events) - ارتفاع النافذة
    pub window_height: i64,
    /// Scroll amount (for mouse wheel) - مقدار التمرير
    pub scroll_amount: i64,
}

impl TrqEvent {
    /// Const default for use in thread_local! initialization
    pub const fn default_const() -> Self {
        Self {
            event_type: TrqEventType::None,
            key_code: 0,
            mouse_x: 0,
            mouse_y: 0,
            mouse_button: 0,
            window_width: 0,
            window_height: 0,
            scroll_amount: 0,
        }
    }
}

impl Default for TrqEvent {
    fn default() -> Self {
        Self::default_const()
    }
}

/// Rectangle structure for drawing
/// Arabic: بنية المستطيل
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TrqRect {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

/// Color structure (RGBA)
/// Arabic: بنية اللون
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TrqColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl From<TrqColor> for Color {
    fn from(c: TrqColor) -> Color {
        Color::RGBA(c.r, c.g, c.b, c.a)
    }
}

// ============================================================================
// Compile-time size assertions for ABI compatibility
// ============================================================================
const _: () = {
    assert!(std::mem::size_of::<TrqEvent>() == 64);
    assert!(std::mem::size_of::<TrqRect>() == 32);
    assert!(std::mem::size_of::<TrqColor>() == 4);
};

// ============================================================================
// Initialization Functions - دوال التهيئة
// ============================================================================

/// Initialize SDL2 subsystems
/// Arabic: هيئ_رسوميات
/// Returns: 1 on success, 0 on failure
#[no_mangle]
pub extern "C" fn trq_sdl_init() -> i64 {
    SDL_CONTEXT.with(|ctx_cell| {
        let mut ctx = ctx_cell.borrow_mut();
        if ctx.is_some() {
            return 1; // Already initialized
        }

        match sdl2::init() {
            Ok(sdl) => {
                // Initialize video subsystem
                if sdl.video().is_err() {
                    return 0;
                }

                // Initialize event pump
                match sdl.event_pump() {
                    Ok(pump) => {
                        EVENT_PUMP.with(|pump_cell| {
                            *pump_cell.borrow_mut() = Some(pump);
                        });
                    }
                    Err(_) => return 0,
                }

                *ctx = Some(sdl);
                1
            }
            Err(_) => 0,
        }
    })
}

/// Cleanup SDL2 and release all resources
/// Arabic: أنهِ_رسوميات
#[no_mangle]
pub extern "C" fn trq_sdl_quit() {
    // Close all windows
    WINDOWS.with(|windows_cell| {
        windows_cell.borrow_mut().clear();
    });

    // Release event pump
    EVENT_PUMP.with(|pump_cell| {
        *pump_cell.borrow_mut() = None;
    });

    // Release SDL context
    SDL_CONTEXT.with(|ctx_cell| {
        *ctx_cell.borrow_mut() = None;
    });
}

/// Check if SDL2 is initialized
/// Arabic: رسوميات_مهيأة
/// Returns: 1 if initialized, 0 otherwise
#[no_mangle]
pub extern "C" fn trq_sdl_is_init() -> i64 {
    SDL_CONTEXT.with(|ctx_cell| if ctx_cell.borrow().is_some() { 1 } else { 0 })
}

// ============================================================================
// Window Functions - دوال النافذة
// ============================================================================

/// Create a new window
/// Arabic: أنشئ_نافذة
/// Parameters:
///   - title: Window title (TrqString)
///   - width: Window width in pixels
///   - height: Window height in pixels
/// Returns: Window handle (>0) on success, -1 on failure
#[no_mangle]
pub extern "C" fn trq_window_create(title: *const TrqString, width: i64, height: i64) -> i64 {
    if title.is_null() {
        return -1;
    }

    // Get title string
    let title_str = unsafe {
        let trq_str = &*title;
        if trq_str.data.is_null() || trq_str.len <= 0 {
            "ترقيم".to_string()
        } else {
            let slice = std::slice::from_raw_parts(trq_str.data, trq_str.len as usize);
            String::from_utf8_lossy(slice).into_owned()
        }
    };

    SDL_CONTEXT.with(|ctx_cell| {
        let ctx = ctx_cell.borrow();
        let sdl = match ctx.as_ref() {
            Some(s) => s,
            None => return -1,
        };

        let video = match sdl.video() {
            Ok(v) => v,
            Err(_) => return -1,
        };

        let window = match video
            .window(&title_str, width as u32, height as u32)
            .position_centered()
            .resizable()
            .build()
        {
            Ok(w) => w,
            Err(_) => return -1,
        };

        let canvas = match window.into_canvas().accelerated().present_vsync().build() {
            Ok(c) => c,
            Err(_) => return -1,
        };

        let texture_creator = canvas.texture_creator();

        // Get next window ID
        let window_id = NEXT_WINDOW_ID.with(|id_cell| {
            let mut id = id_cell.borrow_mut();
            let current_id = *id;
            *id += 1;
            current_id
        });

        // Store window data
        WINDOWS.with(|windows_cell| {
            windows_cell.borrow_mut().insert(
                window_id,
                WindowData {
                    canvas,
                    _texture_creator: texture_creator,
                },
            );
        });

        window_id
    })
}

/// Close and destroy a window
/// Arabic: أغلق_نافذة
/// Parameters:
///   - window_id: Handle returned by trq_window_create
#[no_mangle]
pub extern "C" fn trq_window_close(window_id: i64) {
    WINDOWS.with(|windows_cell| {
        windows_cell.borrow_mut().remove(&window_id);
    });
}

/// Set window title
/// Arabic: عيّن_عنوان_نافذة
#[no_mangle]
pub extern "C" fn trq_window_set_title(window_id: i64, title: *const TrqString) {
    if title.is_null() {
        return;
    }

    let title_str = unsafe {
        let trq_str = &*title;
        if trq_str.data.is_null() || trq_str.len <= 0 {
            return;
        }
        let slice = std::slice::from_raw_parts(trq_str.data, trq_str.len as usize);
        String::from_utf8_lossy(slice).into_owned()
    };

    WINDOWS.with(|windows_cell| {
        let mut windows = windows_cell.borrow_mut();
        if let Some(data) = windows.get_mut(&window_id) {
            let _ = data.canvas.window_mut().set_title(&title_str);
        }
    });
}

/// Get window width
/// Arabic: عرض_نافذة
#[no_mangle]
pub extern "C" fn trq_window_width(window_id: i64) -> i64 {
    WINDOWS.with(|windows_cell| {
        let windows = windows_cell.borrow();
        if let Some(data) = windows.get(&window_id) {
            data.canvas.window().size().0 as i64
        } else {
            0
        }
    })
}

/// Get window height
/// Arabic: ارتفاع_نافذة
#[no_mangle]
pub extern "C" fn trq_window_height(window_id: i64) -> i64 {
    WINDOWS.with(|windows_cell| {
        let windows = windows_cell.borrow();
        if let Some(data) = windows.get(&window_id) {
            data.canvas.window().size().1 as i64
        } else {
            0
        }
    })
}

/// Show the window
/// Arabic: أظهر_نافذة
#[no_mangle]
pub extern "C" fn trq_window_show(window_id: i64) {
    WINDOWS.with(|windows_cell| {
        let mut windows = windows_cell.borrow_mut();
        if let Some(data) = windows.get_mut(&window_id) {
            data.canvas.window_mut().show();
        }
    });
}

/// Hide the window
/// Arabic: أخفِ_نافذة
#[no_mangle]
pub extern "C" fn trq_window_hide(window_id: i64) {
    WINDOWS.with(|windows_cell| {
        let mut windows = windows_cell.borrow_mut();
        if let Some(data) = windows.get_mut(&window_id) {
            data.canvas.window_mut().hide();
        }
    });
}

/// Set window position
/// Arabic: عيّن_موضع_نافذة
#[no_mangle]
pub extern "C" fn trq_window_set_position(window_id: i64, x: i64, y: i64) {
    use sdl2::video::WindowPos;
    WINDOWS.with(|windows_cell| {
        let mut windows = windows_cell.borrow_mut();
        if let Some(data) = windows.get_mut(&window_id) {
            data.canvas.window_mut().set_position(
                WindowPos::Positioned(x as i32),
                WindowPos::Positioned(y as i32),
            );
        }
    });
}

/// Set window size
/// Arabic: عيّن_حجم_نافذة
#[no_mangle]
pub extern "C" fn trq_window_set_size(window_id: i64, width: i64, height: i64) {
    WINDOWS.with(|windows_cell| {
        let mut windows = windows_cell.borrow_mut();
        if let Some(data) = windows.get_mut(&window_id) {
            let _ = data
                .canvas
                .window_mut()
                .set_size(width as u32, height as u32);
        }
    });
}

/// Set window fullscreen mode
/// Arabic: عيّن_ملء_شاشة
/// Parameters:
///   - mode: 0=windowed, 1=fullscreen, 2=fullscreen desktop
#[no_mangle]
pub extern "C" fn trq_window_set_fullscreen(window_id: i64, mode: i64) {
    use sdl2::video::FullscreenType;
    WINDOWS.with(|windows_cell| {
        let mut windows = windows_cell.borrow_mut();
        if let Some(data) = windows.get_mut(&window_id) {
            let fs_type = match mode {
                0 => FullscreenType::Off,
                1 => FullscreenType::True,
                2 => FullscreenType::Desktop,
                _ => FullscreenType::Off,
            };
            let _ = data.canvas.window_mut().set_fullscreen(fs_type);
        }
    });
}

// ============================================================================
// Rendering Functions - دوال الرسم
// ============================================================================

/// Clear the window with the current draw color
/// Arabic: امسح_شاشة
#[no_mangle]
pub extern "C" fn trq_render_clear(window_id: i64) {
    WINDOWS.with(|windows_cell| {
        let mut windows = windows_cell.borrow_mut();
        if let Some(data) = windows.get_mut(&window_id) {
            data.canvas.clear();
        }
    });
}

/// Present the rendered content to the screen
/// Arabic: اعرض_رسم
#[no_mangle]
pub extern "C" fn trq_render_present(window_id: i64) {
    WINDOWS.with(|windows_cell| {
        let mut windows = windows_cell.borrow_mut();
        if let Some(data) = windows.get_mut(&window_id) {
            data.canvas.present();
        }
    });
}

/// Set the drawing color (RGBA)
/// Arabic: عيّن_لون_رسم
#[no_mangle]
pub extern "C" fn trq_render_set_color(window_id: i64, r: u8, g: u8, b: u8, a: u8) {
    WINDOWS.with(|windows_cell| {
        let mut windows = windows_cell.borrow_mut();
        if let Some(data) = windows.get_mut(&window_id) {
            data.canvas.set_draw_color(Color::RGBA(r, g, b, a));
        }
    });
}

/// Set the drawing color using a TrqColor structure
/// Arabic: عيّن_لون_رسم_بنية
#[no_mangle]
pub extern "C" fn trq_render_set_color_struct(window_id: i64, color: TrqColor) {
    trq_render_set_color(window_id, color.r, color.g, color.b, color.a);
}

/// Draw a single point
/// Arabic: ارسم_نقطة
#[no_mangle]
pub extern "C" fn trq_render_point(window_id: i64, x: i64, y: i64) {
    WINDOWS.with(|windows_cell| {
        let mut windows = windows_cell.borrow_mut();
        if let Some(data) = windows.get_mut(&window_id) {
            let _ = data
                .canvas
                .draw_point(sdl2::rect::Point::new(x as i32, y as i32));
        }
    });
}

/// Draw a line between two points
/// Arabic: ارسم_خط
#[no_mangle]
pub extern "C" fn trq_render_line(window_id: i64, x1: i64, y1: i64, x2: i64, y2: i64) {
    WINDOWS.with(|windows_cell| {
        let mut windows = windows_cell.borrow_mut();
        if let Some(data) = windows.get_mut(&window_id) {
            let _ = data.canvas.draw_line(
                sdl2::rect::Point::new(x1 as i32, y1 as i32),
                sdl2::rect::Point::new(x2 as i32, y2 as i32),
            );
        }
    });
}

/// Draw a rectangle outline
/// Arabic: ارسم_مستطيل
#[no_mangle]
pub extern "C" fn trq_render_rect(window_id: i64, x: i64, y: i64, width: i64, height: i64) {
    WINDOWS.with(|windows_cell| {
        let mut windows = windows_cell.borrow_mut();
        if let Some(data) = windows.get_mut(&window_id) {
            let rect = Rect::new(x as i32, y as i32, width as u32, height as u32);
            let _ = data.canvas.draw_rect(rect);
        }
    });
}

/// Draw a filled rectangle
/// Arabic: ارسم_مستطيل_ممتلئ
#[no_mangle]
pub extern "C" fn trq_render_fill_rect(window_id: i64, x: i64, y: i64, width: i64, height: i64) {
    WINDOWS.with(|windows_cell| {
        let mut windows = windows_cell.borrow_mut();
        if let Some(data) = windows.get_mut(&window_id) {
            let rect = Rect::new(x as i32, y as i32, width as u32, height as u32);
            let _ = data.canvas.fill_rect(rect);
        }
    });
}

/// Draw a rectangle using TrqRect structure
/// Arabic: ارسم_مستطيل_بنية
#[no_mangle]
pub extern "C" fn trq_render_rect_struct(window_id: i64, rect: TrqRect) {
    trq_render_rect(window_id, rect.x, rect.y, rect.width, rect.height);
}

/// Draw a filled rectangle using TrqRect structure
/// Arabic: ارسم_مستطيل_ممتلئ_بنية
#[no_mangle]
pub extern "C" fn trq_render_fill_rect_struct(window_id: i64, rect: TrqRect) {
    trq_render_fill_rect(window_id, rect.x, rect.y, rect.width, rect.height);
}

/// Draw a circle outline using midpoint circle algorithm
/// Arabic: ارسم_دائرة
#[no_mangle]
pub extern "C" fn trq_render_circle(window_id: i64, cx: i64, cy: i64, radius: i64) {
    WINDOWS.with(|windows_cell| {
        let mut windows = windows_cell.borrow_mut();
        if let Some(data) = windows.get_mut(&window_id) {
            // Midpoint circle algorithm
            let cx = cx as i32;
            let cy = cy as i32;
            let mut x = radius as i32;
            let mut y = 0i32;
            let mut err = 0i32;

            while x >= y {
                let _ = data
                    .canvas
                    .draw_point(sdl2::rect::Point::new(cx + x, cy + y));
                let _ = data
                    .canvas
                    .draw_point(sdl2::rect::Point::new(cx + y, cy + x));
                let _ = data
                    .canvas
                    .draw_point(sdl2::rect::Point::new(cx - y, cy + x));
                let _ = data
                    .canvas
                    .draw_point(sdl2::rect::Point::new(cx - x, cy + y));
                let _ = data
                    .canvas
                    .draw_point(sdl2::rect::Point::new(cx - x, cy - y));
                let _ = data
                    .canvas
                    .draw_point(sdl2::rect::Point::new(cx - y, cy - x));
                let _ = data
                    .canvas
                    .draw_point(sdl2::rect::Point::new(cx + y, cy - x));
                let _ = data
                    .canvas
                    .draw_point(sdl2::rect::Point::new(cx + x, cy - y));

                y += 1;
                if err <= 0 {
                    err += 2 * y + 1;
                }
                if err > 0 {
                    x -= 1;
                    err -= 2 * x + 1;
                }
            }
        }
    });
}

/// Draw a filled circle
/// Arabic: ارسم_دائرة_ممتلئة
#[no_mangle]
pub extern "C" fn trq_render_fill_circle(window_id: i64, cx: i64, cy: i64, radius: i64) {
    WINDOWS.with(|windows_cell| {
        let mut windows = windows_cell.borrow_mut();
        if let Some(data) = windows.get_mut(&window_id) {
            let cx = cx as i32;
            let cy = cy as i32;
            let radius = radius as i32;

            for dy in -radius..=radius {
                let dx = ((radius * radius - dy * dy) as f64).sqrt() as i32;
                let _ = data.canvas.draw_line(
                    sdl2::rect::Point::new(cx - dx, cy + dy),
                    sdl2::rect::Point::new(cx + dx, cy + dy),
                );
            }
        }
    });
}

/// Draw an ellipse outline
/// Arabic: ارسم_قطع_ناقص
#[no_mangle]
pub extern "C" fn trq_render_ellipse(window_id: i64, cx: i64, cy: i64, rx: i64, ry: i64) {
    WINDOWS.with(|windows_cell| {
        let mut windows = windows_cell.borrow_mut();
        if let Some(data) = windows.get_mut(&window_id) {
            let cx = cx as i32;
            let cy = cy as i32;
            let rx = rx as i32;
            let ry = ry as i32;

            // Bresenham's ellipse algorithm
            let mut x = 0i32;
            let mut y = ry;
            let rx2 = (rx as i64) * (rx as i64);
            let ry2 = (ry as i64) * (ry as i64);
            let mut px = 0i64;
            let mut py = 2 * rx2 * (y as i64);

            // Region 1
            let mut p = ry2 - rx2 * (ry as i64) + rx2 / 4;
            while px < py {
                let _ = data
                    .canvas
                    .draw_point(sdl2::rect::Point::new(cx + x, cy + y));
                let _ = data
                    .canvas
                    .draw_point(sdl2::rect::Point::new(cx - x, cy + y));
                let _ = data
                    .canvas
                    .draw_point(sdl2::rect::Point::new(cx + x, cy - y));
                let _ = data
                    .canvas
                    .draw_point(sdl2::rect::Point::new(cx - x, cy - y));

                x += 1;
                px += 2 * ry2;
                if p < 0 {
                    p += ry2 + px;
                } else {
                    y -= 1;
                    py -= 2 * rx2;
                    p += ry2 + px - py;
                }
            }

            // Region 2
            p = ry2 * ((x as i64) + 1) * ((x as i64) + 1) / 4
                + rx2 * ((y as i64) - 1) * ((y as i64) - 1)
                - rx2 * ry2;
            while y >= 0 {
                let _ = data
                    .canvas
                    .draw_point(sdl2::rect::Point::new(cx + x, cy + y));
                let _ = data
                    .canvas
                    .draw_point(sdl2::rect::Point::new(cx - x, cy + y));
                let _ = data
                    .canvas
                    .draw_point(sdl2::rect::Point::new(cx + x, cy - y));
                let _ = data
                    .canvas
                    .draw_point(sdl2::rect::Point::new(cx - x, cy - y));

                y -= 1;
                py -= 2 * rx2;
                if p > 0 {
                    p += rx2 - py;
                } else {
                    x += 1;
                    px += 2 * ry2;
                    p += rx2 - py + px;
                }
            }
        }
    });
}

// ============================================================================
// Event Handling Functions - دوال معالجة الأحداث
// ============================================================================

/// Poll for the next event
/// Arabic: استطلع_حدث
/// Returns: 1 if an event is available, 0 if no events
#[no_mangle]
pub extern "C" fn trq_event_poll() -> i64 {
    EVENT_PUMP.with(|pump_cell| {
        let mut pump_opt = pump_cell.borrow_mut();
        let pump = match pump_opt.as_mut() {
            Some(p) => p,
            None => return 0,
        };

        match pump.poll_event() {
            Some(event) => {
                let trq_event = convert_sdl_event(&event);
                CURRENT_EVENT.with(|event_cell| {
                    *event_cell.borrow_mut() = trq_event;
                });
                1
            }
            None => 0,
        }
    })
}

/// Wait for the next event (blocking)
/// Arabic: انتظر_حدث
/// Returns: Always 1 (event is available)
#[no_mangle]
pub extern "C" fn trq_event_wait() -> i64 {
    EVENT_PUMP.with(|pump_cell| {
        let mut pump_opt = pump_cell.borrow_mut();
        let pump = match pump_opt.as_mut() {
            Some(p) => p,
            None => return 0,
        };

        let event = pump.wait_event();
        let trq_event = convert_sdl_event(&event);
        CURRENT_EVENT.with(|event_cell| {
            *event_cell.borrow_mut() = trq_event;
        });
        1
    })
}

/// Get the current event (after poll/wait returns 1)
/// Arabic: احصل_على_حدث
#[no_mangle]
pub extern "C" fn trq_event_get() -> TrqEvent {
    CURRENT_EVENT.with(|event_cell| *event_cell.borrow())
}

/// Get current event type
/// Arabic: نوع_حدث
#[no_mangle]
pub extern "C" fn trq_event_type() -> TrqEventType {
    CURRENT_EVENT.with(|event_cell| event_cell.borrow().event_type)
}

/// Get current event key code
/// Arabic: مفتاح_حدث
#[no_mangle]
pub extern "C" fn trq_event_key() -> i64 {
    CURRENT_EVENT.with(|event_cell| event_cell.borrow().key_code)
}

/// Get current event mouse X position
/// Arabic: موضع_فأرة_س
#[no_mangle]
pub extern "C" fn trq_event_mouse_x() -> i64 {
    CURRENT_EVENT.with(|event_cell| event_cell.borrow().mouse_x)
}

/// Get current event mouse Y position
/// Arabic: موضع_فأرة_ص
#[no_mangle]
pub extern "C" fn trq_event_mouse_y() -> i64 {
    CURRENT_EVENT.with(|event_cell| event_cell.borrow().mouse_y)
}

/// Get current event mouse button
/// Arabic: زر_فأرة
#[no_mangle]
pub extern "C" fn trq_event_mouse_button() -> i64 {
    CURRENT_EVENT.with(|event_cell| event_cell.borrow().mouse_button)
}

/// Check if a specific key is pressed
/// Arabic: مفتاح_مضغوط
#[no_mangle]
pub extern "C" fn trq_key_pressed(key_code: i64) -> i64 {
    EVENT_PUMP.with(|pump_cell| {
        let pump_opt = pump_cell.borrow();
        let pump = match pump_opt.as_ref() {
            Some(p) => p,
            None => return 0,
        };

        let keyboard_state = pump.keyboard_state();
        let scancode = sdl2::keyboard::Scancode::from_i32(key_code as i32);
        match scancode {
            Some(sc) => {
                if keyboard_state.is_scancode_pressed(sc) {
                    1
                } else {
                    0
                }
            }
            None => 0,
        }
    })
}

/// Get current mouse position
/// Arabic: موضع_فأرة
/// Returns mouse X in the low 32 bits, Y in the high 32 bits
#[no_mangle]
pub extern "C" fn trq_mouse_position() -> i64 {
    EVENT_PUMP.with(|pump_cell| {
        let pump_opt = pump_cell.borrow();
        let pump = match pump_opt.as_ref() {
            Some(p) => p,
            None => return 0,
        };

        let mouse_state = pump.mouse_state();
        let x = mouse_state.x() as i64;
        let y = mouse_state.y() as i64;
        (y << 32) | (x & 0xFFFFFFFF)
    })
}

/// Get current mouse X position
/// Arabic: موضع_فأرة_حالي_س
#[no_mangle]
pub extern "C" fn trq_mouse_x() -> i64 {
    EVENT_PUMP.with(|pump_cell| {
        let pump_opt = pump_cell.borrow();
        let pump = match pump_opt.as_ref() {
            Some(p) => p,
            None => return 0,
        };

        pump.mouse_state().x() as i64
    })
}

/// Get current mouse Y position
/// Arabic: موضع_فأرة_حالي_ص
#[no_mangle]
pub extern "C" fn trq_mouse_y() -> i64 {
    EVENT_PUMP.with(|pump_cell| {
        let pump_opt = pump_cell.borrow();
        let pump = match pump_opt.as_ref() {
            Some(p) => p,
            None => return 0,
        };

        pump.mouse_state().y() as i64
    })
}

/// Check if a mouse button is pressed
/// Arabic: زر_فأرة_مضغوط
/// Parameters: button (1=left, 2=middle, 3=right)
#[no_mangle]
pub extern "C" fn trq_mouse_button_pressed(button: i64) -> i64 {
    EVENT_PUMP.with(|pump_cell| {
        let pump_opt = pump_cell.borrow();
        let pump = match pump_opt.as_ref() {
            Some(p) => p,
            None => return 0,
        };

        let mouse_state = pump.mouse_state();
        let is_pressed = match button {
            1 => mouse_state.left(),
            2 => mouse_state.middle(),
            3 => mouse_state.right(),
            _ => false,
        };
        if is_pressed {
            1
        } else {
            0
        }
    })
}

// ============================================================================
// Color Utility Functions - دوال الألوان
// ============================================================================

/// Create an RGB color (alpha = 255)
/// Arabic: لون_من_ر_خ_ز
#[no_mangle]
pub extern "C" fn trq_color_rgb(r: u8, g: u8, b: u8) -> TrqColor {
    TrqColor { r, g, b, a: 255 }
}

/// Create an RGBA color
/// Arabic: لون_من_ر_خ_ز_ش
#[no_mangle]
pub extern "C" fn trq_color_rgba(r: u8, g: u8, b: u8, a: u8) -> TrqColor {
    TrqColor { r, g, b, a }
}

/// Create color from HSL values
/// Arabic: لون_من_هسل
/// Parameters: h (0-360), s (0-100), l (0-100)
#[no_mangle]
pub extern "C" fn trq_color_hsl(h: i64, s: i64, l: i64) -> TrqColor {
    let h = (h % 360) as f64 / 360.0;
    let s = (s.clamp(0, 100)) as f64 / 100.0;
    let l = (l.clamp(0, 100)) as f64 / 100.0;

    if s == 0.0 {
        let gray = (l * 255.0) as u8;
        return TrqColor {
            r: gray,
            g: gray,
            b: gray,
            a: 255,
        };
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            return p + (q - p) * 6.0 * t;
        }
        if t < 1.0 / 2.0 {
            return q;
        }
        if t < 2.0 / 3.0 {
            return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
        }
        p
    }

    let r = (hue_to_rgb(p, q, h + 1.0 / 3.0) * 255.0) as u8;
    let g = (hue_to_rgb(p, q, h) * 255.0) as u8;
    let b = (hue_to_rgb(p, q, h - 1.0 / 3.0) * 255.0) as u8;

    TrqColor { r, g, b, a: 255 }
}

// ============================================================================
// Predefined Colors - الألوان المعرّفة مسبقاً
// ============================================================================

/// Black color - أسود
#[no_mangle]
pub extern "C" fn trq_color_black() -> TrqColor {
    TrqColor {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    }
}

/// White color - أبيض
#[no_mangle]
pub extern "C" fn trq_color_white() -> TrqColor {
    TrqColor {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    }
}

/// Red color - أحمر
#[no_mangle]
pub extern "C" fn trq_color_red() -> TrqColor {
    TrqColor {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    }
}

/// Green color - أخضر
#[no_mangle]
pub extern "C" fn trq_color_green() -> TrqColor {
    TrqColor {
        r: 0,
        g: 255,
        b: 0,
        a: 255,
    }
}

/// Blue color - أزرق
#[no_mangle]
pub extern "C" fn trq_color_blue() -> TrqColor {
    TrqColor {
        r: 0,
        g: 0,
        b: 255,
        a: 255,
    }
}

/// Yellow color - أصفر
#[no_mangle]
pub extern "C" fn trq_color_yellow() -> TrqColor {
    TrqColor {
        r: 255,
        g: 255,
        b: 0,
        a: 255,
    }
}

/// Cyan color - سماوي
#[no_mangle]
pub extern "C" fn trq_color_cyan() -> TrqColor {
    TrqColor {
        r: 0,
        g: 255,
        b: 255,
        a: 255,
    }
}

/// Magenta color - أرجواني
#[no_mangle]
pub extern "C" fn trq_color_magenta() -> TrqColor {
    TrqColor {
        r: 255,
        g: 0,
        b: 255,
        a: 255,
    }
}

/// Orange color - برتقالي
#[no_mangle]
pub extern "C" fn trq_color_orange() -> TrqColor {
    TrqColor {
        r: 255,
        g: 165,
        b: 0,
        a: 255,
    }
}

/// Gray color - رمادي
#[no_mangle]
pub extern "C" fn trq_color_gray() -> TrqColor {
    TrqColor {
        r: 128,
        g: 128,
        b: 128,
        a: 255,
    }
}

// ============================================================================
// Timing Functions - دوال التوقيت
// ============================================================================

/// Delay execution for the specified milliseconds
/// Arabic: تأخير
#[no_mangle]
pub extern "C" fn trq_delay(ms: i64) {
    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
}

/// Get the number of milliseconds since SDL initialization
/// Arabic: وقت_تشغيل
#[no_mangle]
pub extern "C" fn trq_get_ticks() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis() as i64,
        Err(_) => 0,
    }
}

// ============================================================================
// Keyboard Constants - ثوابت لوحة المفاتيح
// ============================================================================

// Arrow keys
#[no_mangle]
pub static TRQ_KEY_UP: i64 = 82;
#[no_mangle]
pub static TRQ_KEY_DOWN: i64 = 81;
#[no_mangle]
pub static TRQ_KEY_LEFT: i64 = 80;
#[no_mangle]
pub static TRQ_KEY_RIGHT: i64 = 79;

// Special keys
#[no_mangle]
pub static TRQ_KEY_SPACE: i64 = 44;
#[no_mangle]
pub static TRQ_KEY_ENTER: i64 = 40;
#[no_mangle]
pub static TRQ_KEY_ESCAPE: i64 = 41;
#[no_mangle]
pub static TRQ_KEY_BACKSPACE: i64 = 42;
#[no_mangle]
pub static TRQ_KEY_TAB: i64 = 43;

// Letter keys (A-Z)
#[no_mangle]
pub static TRQ_KEY_A: i64 = 4;
#[no_mangle]
pub static TRQ_KEY_B: i64 = 5;
#[no_mangle]
pub static TRQ_KEY_C: i64 = 6;
#[no_mangle]
pub static TRQ_KEY_D: i64 = 7;
#[no_mangle]
pub static TRQ_KEY_E: i64 = 8;
#[no_mangle]
pub static TRQ_KEY_F: i64 = 9;
#[no_mangle]
pub static TRQ_KEY_G: i64 = 10;
#[no_mangle]
pub static TRQ_KEY_H: i64 = 11;
#[no_mangle]
pub static TRQ_KEY_I: i64 = 12;
#[no_mangle]
pub static TRQ_KEY_J: i64 = 13;
#[no_mangle]
pub static TRQ_KEY_K: i64 = 14;
#[no_mangle]
pub static TRQ_KEY_L: i64 = 15;
#[no_mangle]
pub static TRQ_KEY_M: i64 = 16;
#[no_mangle]
pub static TRQ_KEY_N: i64 = 17;
#[no_mangle]
pub static TRQ_KEY_O: i64 = 18;
#[no_mangle]
pub static TRQ_KEY_P: i64 = 19;
#[no_mangle]
pub static TRQ_KEY_Q: i64 = 20;
#[no_mangle]
pub static TRQ_KEY_R: i64 = 21;
#[no_mangle]
pub static TRQ_KEY_S: i64 = 22;
#[no_mangle]
pub static TRQ_KEY_T: i64 = 23;
#[no_mangle]
pub static TRQ_KEY_U: i64 = 24;
#[no_mangle]
pub static TRQ_KEY_V: i64 = 25;
#[no_mangle]
pub static TRQ_KEY_W: i64 = 26;
#[no_mangle]
pub static TRQ_KEY_X: i64 = 27;
#[no_mangle]
pub static TRQ_KEY_Y: i64 = 28;
#[no_mangle]
pub static TRQ_KEY_Z: i64 = 29;

// Number keys (0-9)
#[no_mangle]
pub static TRQ_KEY_0: i64 = 39;
#[no_mangle]
pub static TRQ_KEY_1: i64 = 30;
#[no_mangle]
pub static TRQ_KEY_2: i64 = 31;
#[no_mangle]
pub static TRQ_KEY_3: i64 = 32;
#[no_mangle]
pub static TRQ_KEY_4: i64 = 33;
#[no_mangle]
pub static TRQ_KEY_5: i64 = 34;
#[no_mangle]
pub static TRQ_KEY_6: i64 = 35;
#[no_mangle]
pub static TRQ_KEY_7: i64 = 36;
#[no_mangle]
pub static TRQ_KEY_8: i64 = 37;
#[no_mangle]
pub static TRQ_KEY_9: i64 = 38;

// ============================================================================
// Helper Functions (Internal)
// ============================================================================

/// Convert SDL2 event to TrqEvent
fn convert_sdl_event(event: &Event) -> TrqEvent {
    match event {
        Event::Quit { .. } => TrqEvent {
            event_type: TrqEventType::Quit,
            ..Default::default()
        },

        Event::KeyDown {
            scancode: Some(sc), ..
        } => TrqEvent {
            event_type: TrqEventType::KeyDown,
            key_code: *sc as i64,
            ..Default::default()
        },

        Event::KeyUp {
            scancode: Some(sc), ..
        } => TrqEvent {
            event_type: TrqEventType::KeyUp,
            key_code: *sc as i64,
            ..Default::default()
        },

        Event::MouseMotion { x, y, .. } => TrqEvent {
            event_type: TrqEventType::MouseMotion,
            mouse_x: *x as i64,
            mouse_y: *y as i64,
            ..Default::default()
        },

        Event::MouseButtonDown {
            mouse_btn, x, y, ..
        } => TrqEvent {
            event_type: TrqEventType::MouseButtonDown,
            mouse_x: *x as i64,
            mouse_y: *y as i64,
            mouse_button: mouse_button_to_i64(*mouse_btn),
            ..Default::default()
        },

        Event::MouseButtonUp {
            mouse_btn, x, y, ..
        } => TrqEvent {
            event_type: TrqEventType::MouseButtonUp,
            mouse_x: *x as i64,
            mouse_y: *y as i64,
            mouse_button: mouse_button_to_i64(*mouse_btn),
            ..Default::default()
        },

        Event::MouseWheel { y, .. } => TrqEvent {
            event_type: TrqEventType::MouseWheel,
            scroll_amount: *y as i64,
            ..Default::default()
        },

        Event::Window {
            win_event: sdl2::event::WindowEvent::Resized(w, h),
            ..
        } => TrqEvent {
            event_type: TrqEventType::WindowResized,
            window_width: *w as i64,
            window_height: *h as i64,
            ..Default::default()
        },

        _ => TrqEvent::default(),
    }
}

/// Convert SDL2 mouse button to i64
fn mouse_button_to_i64(btn: MouseButton) -> i64 {
    match btn {
        MouseButton::Left => 1,
        MouseButton::Middle => 2,
        MouseButton::Right => 3,
        MouseButton::X1 => 4,
        MouseButton::X2 => 5,
        MouseButton::Unknown => 0,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_creation() {
        let c = trq_color_rgb(255, 128, 64);
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 128);
        assert_eq!(c.b, 64);
        assert_eq!(c.a, 255);
    }

    #[test]
    fn test_color_rgba() {
        let c = trq_color_rgba(100, 150, 200, 128);
        assert_eq!(c.r, 100);
        assert_eq!(c.g, 150);
        assert_eq!(c.b, 200);
        assert_eq!(c.a, 128);
    }

    #[test]
    fn test_predefined_colors() {
        let black = trq_color_black();
        assert_eq!(black.r, 0);
        assert_eq!(black.g, 0);
        assert_eq!(black.b, 0);

        let white = trq_color_white();
        assert_eq!(white.r, 255);
        assert_eq!(white.g, 255);
        assert_eq!(white.b, 255);
    }

    #[test]
    fn test_event_default() {
        let e = TrqEvent::default();
        assert_eq!(e.event_type, TrqEventType::None);
        assert_eq!(e.key_code, 0);
    }

    #[test]
    fn test_event_default_const() {
        let e = TrqEvent::default_const();
        assert_eq!(e.event_type, TrqEventType::None);
        assert_eq!(e.key_code, 0);
    }

    #[test]
    fn test_struct_sizes() {
        assert_eq!(std::mem::size_of::<TrqEvent>(), 64);
        assert_eq!(std::mem::size_of::<TrqRect>(), 32);
        assert_eq!(std::mem::size_of::<TrqColor>(), 4);
    }
}
