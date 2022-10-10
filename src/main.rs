fn main() {
    let v: Vec<u8> = vec![
        134, 170, 161, 82, 215, 63, 105, 98, 17, 135, 43, 67, 139, 168, 24, 41, 7, 40, 89, 147, 11,
        126, 171, 43, 178, 40, 136, 122, 234, 79, 117, 58, 214, 140, 57, 155, 14, 161, 67, 50, 146,
        143, 86, 228, 129, 114, 42, 5, 199, 213, 56, 57, 160, 63, 201, 33, 203, 4, 101, 160, 75,
        90, 103, 141, 121, 117, 141, 209, 59, 121, 169, 173, 155, 120, 24, 45, 169, 151, 147, 79,
        153, 244, 91, 23, 204, 116, 3, 101, 216, 98, 130, 176, 192, 0, 212, 143, 5, 54, 115, 85,
        222, 163, 142, 164, 210, 35, 207, 73, 150, 47, 156, 201, 186, 148, 225, 1, 60, 39, 67, 19,
        86, 110, 96, 235, 54, 50, 81, 2, 194, 17, 151, 242, 158, 137, 109, 37, 105, 216, 172, 75,
        114, 215, 42, 171, 89, 25, 203, 99, 57, 107, 150, 164, 245, 178, 222, 221, 243, 123, 93,
        253, 86, 5, 58, 97, 100, 80, 216, 32, 34, 234, 129, 169, 142, 20, 141, 13, 3, 226, 13, 129,
        173, 73, 71, 195, 26, 239, 93, 4, 68, 246, 240, 168, 90, 169, 121, 221, 106, 185, 117, 199,
        98, 1, 25, 71, 51, 211, 120, 101, 178, 179, 201, 97, 102, 52, 56, 226, 98, 49, 249, 189,
        108, 236, 58, 68, 125, 224, 151, 245, 70, 184, 22, 228, 208, 70, 148, 16, 55, 159, 205,
        234, 63, 212, 113, 188, 147, 213, 1, 162, 60, 201, 244, 103, 226, 103, 184, 195, 127, 247,
        33, 224, 246, 159, 38, 211, 209, 124, 52, 234, 231, 150, 168, 175, 211, 238, 38, 25, 83,
        23, 207, 198, 213, 171, 104, 99, 216, 223, 176, 241, 140, 222, 202, 197, 252, 46, 51, 215,
        94, 210, 99, 220, 105, 247, 176, 253, 60, 78, 25, 48, 139, 36, 249, 5, 67, 71, 145, 83, 77,
        23, 133, 68, 223, 56, 178, 30, 50, 139, 182, 22, 42, 89, 0, 120, 163, 185, 183, 197, 254,
        97, 228, 25, 234, 104, 98, 124, 215, 33, 90, 202, 100, 29, 251, 3, 93, 182, 235, 205, 161,
        138, 67, 112, 64, 100, 134, 68, 93, 105, 25, 90, 24, 230, 180, 57, 84, 175, 25, 142, 214,
        163, 152, 78, 83, 127, 230, 135, 128, 128, 171, 89, 18, 121, 95, 185, 125, 46, 25, 194,
        160, 3, 57, 2, 226, 224, 174, 31, 113, 140, 184, 11, 13, 190, 96, 29, 143, 241, 163, 128,
        245, 219, 109, 130, 221, 249, 186, 195, 247, 198, 255, 160, 126, 205, 230, 224, 54, 187,
        135, 229, 29, 202, 16, 164, 188, 157, 137, 54, 193, 39, 227, 225, 225, 103, 156, 19, 138,
        205, 148, 59, 243, 222, 116, 60, 163, 158, 164, 80, 124, 79, 180, 39, 9, 44, 216, 88, 50,
        102, 141, 233, 135, 84, 248, 136, 157, 181, 179, 46, 72, 120, 67, 25, 91, 121, 36, 204,
        124, 75, 222, 110, 0, 40, 101, 136, 62, 42, 155, 144, 229, 205, 198, 53, 206, 55, 162, 8,
    ];
    let t: Vec<u8> = vec![
        48, 130, 1, 34, 48, 13, 6, 9, 42, 134, 72, 134, 247, 13, 1, 1, 1, 5, 0, 3, 130, 1, 15, 0,
        48, 130, 1, 10, 2, 130, 1, 1, 0, 229, 6, 138, 205, 38, 50, 227, 244, 212, 139, 222, 110,
        193, 31, 88, 181, 105, 178, 216, 98, 135, 172, 244, 119, 137, 92, 62, 107, 35, 30, 7, 214,
        124, 120, 245, 190, 88, 173, 96, 85, 186, 29, 198, 79, 148, 119, 102, 233, 54, 183, 163,
        156, 225, 223, 63, 123, 230, 249, 202, 252, 200, 48, 139, 222, 161, 187, 91, 79, 62, 211,
        229, 226, 80, 134, 236, 174, 147, 79, 209, 208, 142, 173, 73, 214, 139, 61, 196, 104, 38,
        236, 57, 154, 140, 122, 226, 16, 55, 82, 13, 101, 91, 252, 253, 128, 157, 128, 221, 219,
        152, 135, 146, 175, 117, 103, 86, 222, 32, 167, 202, 114, 116, 245, 183, 113, 196, 217, 50,
        219, 151, 9, 104, 38, 212, 166, 208, 100, 34, 41, 76, 213, 66, 79, 139, 149, 155, 156, 32,
        79, 113, 61, 126, 3, 181, 102, 61, 98, 187, 140, 5, 71, 65, 94, 57, 204, 92, 85, 136, 86,
        192, 101, 7, 172, 68, 213, 87, 238, 185, 55, 253, 178, 121, 89, 98, 213, 94, 74, 12, 29,
        164, 116, 92, 110, 104, 150, 47, 53, 202, 110, 237, 172, 167, 88, 3, 87, 252, 41, 25, 125,
        137, 103, 117, 57, 107, 13, 93, 107, 224, 183, 236, 31, 5, 252, 65, 216, 109, 5, 56, 238,
        87, 54, 45, 220, 60, 167, 23, 87, 103, 61, 117, 142, 107, 28, 182, 224, 106, 36, 158, 94,
        251, 145, 208, 109, 208, 169, 54, 151, 2, 3, 1, 0, 1,
    ];
    print_nice_arr("Key", t.iter().map(|x| *x as i8).collect::<Vec<i8>>());
    print_nice_arr("Sig", v.iter().map(|x| *x as i8).collect::<Vec<i8>>());
}

pub fn print_nice_arr<S: Into<String>>(name: S, vec: Vec<i8>) {
    let name = name.into();
    println!("==== {} Header ====", name);
    println!();
    vec.chunks(10).for_each(|chunk| {
        let fmt = format!("{:?}", chunk);
        println!("{},", &fmt[1..fmt.len() - 1]);
    });
    println!();
    println!("==== {} Footer ====", name);
}