use sdl2::{pixels::Color, rect::Rect, render::WindowCanvas};

pub fn interpolate_color(start: Color, stop: Color, progress: f32) -> Color {
    let r = (start.r as f32 + (stop.r as f32 - start.r as f32) * progress) as u8;
    let g = (start.g as f32 + (stop.g as f32 - start.g as f32) * progress) as u8;
    let b = (start.b as f32 + (stop.b as f32 - start.b as f32) * progress) as u8;
    let a = (start.a as f32 + (stop.a as f32 - start.a as f32) * progress) as u8;
    Color::RGBA(r, g, b, a)
}

pub fn render_gradient_border(
    canvas: &mut WindowCanvas,
    bound: Rect,
    outer_color: Color,
    inner_color: Color,
    border_width: u16,
    steps: u16,
) {
    let step_width = border_width / (steps + 1);
    let step_width_u32 = u32::from(step_width);
    let step_width_i32 = i32::from(step_width);

    for i in 0..steps {
        let color = if steps <= 1 {
            outer_color
        } else {
            interpolate_color(outer_color, inner_color, i as f32 / (steps - 1) as f32)
        };
        canvas.set_draw_color(color);
        canvas // top
            .fill_rect(Rect::new(
                bound.x + i32::from(i) * step_width_i32,
                bound.y + i32::from(i) * step_width_i32,
                bound.width() - u32::from(i) * step_width_u32 * 2,
                step_width_u32,
            ))
            .unwrap();
        canvas // right
            .fill_rect(Rect::new(
                bound.x + bound.w - (1 + i32::from(i)) * step_width_i32,
                bound.y + i32::from(i) * step_width_i32,
                step_width_u32,
                bound
                    .height()
                    .checked_sub(u32::from(i) * step_width_u32 * 2)
                    .unwrap_or(bound.height()),
            ))
            .unwrap();
        canvas // bottom
            .fill_rect(Rect::new(
                bound.x + i32::from(i) * step_width_i32,
                bound.y + bound.h - (1 + i32::from(i)) * step_width_i32,
                bound.width() - u32::from(i) * step_width_u32 * 2,
                step_width_u32,
            ))
            .unwrap();
        canvas // left
            .fill_rect(Rect::new(
                bound.x + i32::from(i) * step_width_i32,
                bound.y + i32::from(i) * step_width_i32,
                step_width_u32,
                bound
                    .height()
                    .checked_sub(u32::from(i) * step_width_u32 * 2)
                    .unwrap_or(bound.height()),
            ))
            .unwrap();
    }
}
