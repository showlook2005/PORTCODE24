use chrono::TimeZone;
use chrono_tz::Tz;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use cron_rs::parser::parse_standard;

fn bench_next_calculation(c: &mut Criterion) {
    let sched = parse_standard("0/15 8-18 * * Mon-Fri").unwrap();
    let seed = Tz::UTC.timestamp_opt(1600000000, 0).unwrap();

    c.bench_function("spec_next_calculation_p99", |b| {
        b.iter(|| {
            black_box(sched.next(black_box(seed)));
        });
    });
}

criterion_group!(benches, bench_next_calculation);
criterion_main!(benches);
