use super::{LOOP_BUTTON_COLOR, NORMAL_BUTTON_COLOR, SLIDER_COLOR};
pub fn create_icon_data() -> Vec<u8> {
    let nb = vec![
        (NORMAL_BUTTON_COLOR.base_color[0] * 255.0) as u8,
        (NORMAL_BUTTON_COLOR.base_color[1] * 255.0) as u8,
        (NORMAL_BUTTON_COLOR.base_color[2] * 255.0) as u8,
        255,
    ];
    let nh = vec![
        (NORMAL_BUTTON_COLOR.hover_color[0] * 255.0) as u8,
        (NORMAL_BUTTON_COLOR.hover_color[1] * 255.0) as u8,
        (NORMAL_BUTTON_COLOR.hover_color[2] * 255.0) as u8,
        255,
    ];
    let lb = vec![
        (LOOP_BUTTON_COLOR.base_color[0] * 255.0) as u8,
        (LOOP_BUTTON_COLOR.base_color[1] * 255.0) as u8,
        (LOOP_BUTTON_COLOR.base_color[2] * 255.0) as u8,
        255,
    ];
    let sb = vec![
        (SLIDER_COLOR.base_color[0] * 255.0) as u8,
        (SLIDER_COLOR.base_color[1] * 255.0) as u8,
        (SLIDER_COLOR.base_color[2] * 255.0) as u8,
        255,
    ];
    let sh = vec![
        (SLIDER_COLOR.hover_color[0] * 255.0) as u8,
        (SLIDER_COLOR.hover_color[1] * 255.0) as u8,
        (SLIDER_COLOR.hover_color[2] * 255.0) as u8,
        255,
    ];
    
    let pixels = vec![
        nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), lb.clone(), lb.clone(), lb.clone(), lb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), 
        nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), lb.clone(), lb.clone(), lb.clone(), lb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), 
        nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), lb.clone(), lb.clone(), lb.clone(), lb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), 
        nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), lb.clone(), lb.clone(), lb.clone(), lb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(),  
        sh.clone(), sh.clone(), sh.clone(), sh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), 
        sh.clone(), sh.clone(), sh.clone(), sh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), 
        sh.clone(), sh.clone(), sh.clone(), sh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), 
        sh.clone(), sh.clone(), sh.clone(), sh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), nh.clone(), 
        sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), 
        sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), 
        sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), 
        sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), sb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), 
        sb.clone(), sb.clone(), sb.clone(), sb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(),
        sb.clone(), sb.clone(), sb.clone(), sb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(),
        sb.clone(), sb.clone(), sb.clone(), sb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(),
        sb.clone(), sb.clone(), sb.clone(), sb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(), nb.clone(),
    ];
    pixels.into_iter().flatten().collect::<Vec<u8>>()
}
