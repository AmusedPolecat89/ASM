use asm_web::figures::{render_histogram_svg, FigureConfig};

#[test]
fn histogram_is_deterministic() {
    let config = FigureConfig {
        width: 100,
        height: 50,
        bins: 4,
    };
    let values = vec![0.1, 0.2, 0.4, 0.5];
    let svg_a = render_histogram_svg(&values, &config);
    let svg_b = render_histogram_svg(&values, &config);
    assert_eq!(svg_a, svg_b);
    assert!(svg_a.contains("rect"));
}
