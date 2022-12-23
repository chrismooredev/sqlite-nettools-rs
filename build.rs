use std::path::Path;

const DB_LINK: &str = "https://gitlab.com/wireshark/wireshark/-/raw/master/manuf";

fn main() {
    // download Wireshark OUI database into OUT_DIR to embed within extension
    let db_data: String = ureq::get(DB_LINK)
        .call().unwrap()
        .into_string().unwrap();
    
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let db_path = Path::new(&out_dir).join("wireshark_oui_db.txt");

    std::fs::write(&db_path, &db_data).expect("unable to write wireshark db file");
}
