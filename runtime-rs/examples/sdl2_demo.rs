//! SDL2 Demo - Tests the Tarqeem SDL2 bindings
//!
//! This example demonstrates the SDL2 graphics library bindings for Tarqeem.
//! Run with: cargo run --example sdl2_demo --features graphics

use trq::sdl2::*;
use trq::types::TrqString;

fn main() {
    // Initialize SDL2
    let init_result = trq_sdl_init();
    if init_result == 0 {
        eprintln!("Failed to initialize SDL2");
        return;
    }
    println!("SDL2 initialized successfully!");

    // Create a window with Arabic title
    let title_str = "مثال ترقيم - رسوميات SDL2";
    let mut title_bytes = title_str.as_bytes().to_vec();
    title_bytes.push(0); // null-terminate
    let title = TrqString {
        len: (title_bytes.len() - 1) as i64, // exclude null terminator
        cap: title_bytes.len() as i64,
        data: title_bytes.as_mut_ptr(),
    };
    // Keep bytes alive for the duration of window creation
    let _title_keep = title_bytes;

    let window = trq_window_create(&title, 800, 600);
    if window < 0 {
        eprintln!("Failed to create window");
        trq_sdl_quit();
        return;
    }
    println!("Window created with ID: {}", window);

    // Draw some frames
    let mut frame_count = 0;
    let max_frames = 120; // Run for ~2 seconds at 60fps

    while frame_count < max_frames {
        // Poll events
        while trq_event_poll() == 1 {
            let event_type = trq_event_type();
            if event_type == TrqEventType::Quit {
                println!("Quit event received");
                frame_count = max_frames;
                break;
            }
            if event_type == TrqEventType::KeyDown {
                let key = trq_event_key();
                if key == TRQ_KEY_ESCAPE {
                    println!("Escape pressed");
                    frame_count = max_frames;
                    break;
                }
            }
        }

        // Clear screen with dark blue
        let bg = trq_color_rgb(20, 30, 60);
        trq_render_set_color_struct(window, bg);
        trq_render_clear(window);

        // Draw gradient background
        for y in (0..600).step_by(20) {
            let intensity = 20 + (y / 15) as u8;
            trq_render_set_color(window, intensity, intensity, 50, 255);
            trq_render_fill_rect(window, 0, y as i64, 800, 20);
        }

        // Draw title banner
        let white = trq_color_white();
        trq_render_set_color_struct(window, white);
        trq_render_fill_rect(window, 250, 30, 300, 60);

        let blue = trq_color_blue();
        trq_render_set_color_struct(window, blue);
        trq_render_rect(window, 248, 28, 304, 64);

        // Draw red rectangle
        let red = trq_color_red();
        trq_render_set_color_struct(window, red);
        trq_render_fill_rect(window, 100, 150, 150, 100);

        // Draw green rectangle
        let green = trq_color_green();
        trq_render_set_color_struct(window, green);
        trq_render_fill_rect(window, 325, 150, 150, 100);

        // Draw blue rectangle
        trq_render_set_color_struct(window, blue);
        trq_render_fill_rect(window, 550, 150, 150, 100);

        // Draw circles
        let yellow = trq_color_yellow();
        trq_render_set_color_struct(window, yellow);
        trq_render_fill_circle(window, 175, 350, 60);

        let orange = trq_color_orange();
        trq_render_set_color_struct(window, orange);
        trq_render_fill_circle(window, 400, 350, 60);

        let magenta = trq_color_rgb(255, 0, 255);
        trq_render_set_color_struct(window, magenta);
        trq_render_fill_circle(window, 625, 350, 60);

        // Draw circle outlines
        trq_render_set_color_struct(window, white);
        trq_render_circle(window, 175, 350, 65);
        trq_render_circle(window, 400, 350, 65);
        trq_render_circle(window, 625, 350, 65);

        // Draw decorative lines
        let cyan = trq_color_cyan();
        trq_render_set_color_struct(window, cyan);
        trq_render_line(window, 50, 450, 750, 450);
        trq_render_line(window, 50, 460, 750, 460);

        // Draw animated pattern
        let offset = (frame_count % 40) as i64;
        trq_render_set_color(window, 200, 200, 255, 255);
        for x in (offset..800).step_by(40) {
            trq_render_fill_rect(window, x, 500, 20, 20);
        }

        // Draw frame border
        trq_render_set_color_struct(window, white);
        trq_render_rect(window, 10, 10, 780, 580);

        // Present the frame
        trq_render_present(window);

        // Delay for ~60fps
        trq_delay(16);
        frame_count += 1;
    }

    // Cleanup
    trq_window_close(window);
    trq_sdl_quit();
    println!("SDL2 demo completed successfully!");
}
