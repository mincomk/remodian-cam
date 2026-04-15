const TEMPLATES_BYTES: [&'static [u8; 35]; 10] = [
    include_bytes!("../../templates/0.bin"),
    include_bytes!("../../templates/1.bin"),
    include_bytes!("../../templates/2.bin"),
    include_bytes!("../../templates/3.bin"),
    include_bytes!("../../templates/4.bin"),
    include_bytes!("../../templates/5.bin"),
    include_bytes!("../../templates/6.bin"),
    include_bytes!("../../templates/7.bin"),
    include_bytes!("../../templates/8.bin"),
    include_bytes!("../../templates/9.bin"),
];

/// Load templates, upsampling each 5×7 source to 10×14 by 2× nearest-neighbor.
/// Each original cell maps to a 2×2 block of new cells.
pub const fn load_templates() -> [[bool; 140]; 10] {
    let mut templates = [[false; 140]; 10];
    let mut i = 0;
    while i < 10 {
        let bytes = TEMPLATES_BYTES[i];
        let mut cy = 0;
        while cy < 7 {
            let mut cx = 0;
            while cx < 5 {
                let val = bytes[cy * 5 + cx] != 0;
                let dy = cy * 2;
                let dx = cx * 2;
                templates[i][dy * 10 + dx] = val;
                templates[i][dy * 10 + dx + 1] = val;
                templates[i][(dy + 1) * 10 + dx] = val;
                templates[i][(dy + 1) * 10 + dx + 1] = val;
                cx += 1;
            }
            cy += 1;
        }
        i += 1;
    }
    templates
}
