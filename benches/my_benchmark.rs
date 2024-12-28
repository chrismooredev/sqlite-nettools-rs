use criterion::{black_box, criterion_group, criterion_main, Criterion};
use eui48::MacAddress;
use sqlite3_nettools::{mac::MacStyle, oui::Oui};


// #[bench]
// fn format_types(b: &mut Bencher<'_>) -> impl std::process::Termination {
//     const CASES: [(MacStyle, bool); 5] = [
//         (MacStyle::Plain, false),
//         (MacStyle::Plain, true),
//         (MacStyle::Dashed, false),
//         (MacStyle::InterfaceId, true),
//         (MacStyle::Colon, true),
//     ];

//     const MAC: MacAddress = MacAddress::from_bytes(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]).unwrap();

//     b.iter(|| {
//         for (st, cap) in CASES.iter() {
//             st.format(MAC, cap);
//         }
//     })
// }


fn criterion_benchmark(c: &mut Criterion) {
    const CASES: [(MacStyle, bool); 5] = [
        (MacStyle::Plain, false),
        (MacStyle::Plain, true),
        (MacStyle::Dashed, false),
        (MacStyle::InterfaceId, true),
        (MacStyle::Colon, true),
    ];

    
    let mac: MacAddress = Oui::from_int(0x0000AABBCCDDEEFF).unwrap().as_mac();

    c.bench_function("stringify macs", |b| b.iter(|| CASES.iter().map(|(st, cap)| {
        black_box(st.format(black_box(mac), black_box(*cap)))
    })));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);