use std::path::Path;

const DB_LINK_URLS: &[&str] = &[
    "https://gitlab.com/wireshark/wireshark/-/raw/master/manuf",
    "https://www.wireshark.org/download/automated/data/manuf",
];

#[derive(Debug)]
struct DatabaseSourceError(Vec<(&'static str, ureq::Error)>);

impl std::error::Error for DatabaseSourceError {}
impl std::fmt::Display for DatabaseSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "unable to fetch Wireshark OUI Database from any of the following links: ")?;
        for (link, error) in &self.0 {
            write!(f, "\t{}: {}\n", link, error)?;
        }
        Ok(())
    }
}

fn fetch_db_data() -> Result<String, DatabaseSourceError> {
    let mut error = Vec::new();
    for link in DB_LINK_URLS {
        match ureq::get(link).call() {
            Ok(resp) => {
                let text = resp.into_string().expect("unable to successfully parse fetch Wireshark OUI Database as UTF8");
                return Ok(text);
            },
            Err(e) => {
                error.push((*link, e))
            },
        };
    };
    Err(DatabaseSourceError(error))
}

fn main() {
    // download Wireshark OUI database into OUT_DIR to embed within extension
    let db_data = fetch_db_data().expect("unable to fetch Wireshark OUI Database");

    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let db_path = Path::new(&out_dir).join("wireshark_oui_db.txt");

    std::fs::write(db_path, db_data).expect("unable to write wireshark db file");
}
