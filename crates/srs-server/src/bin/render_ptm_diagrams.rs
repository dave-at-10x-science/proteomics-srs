//! Render the checked PTM connectivity schematics without a chemistry service or web dependency.
//!
//! Run with an optional output directory; the default is web/public/study/ptms.
//! These drawings show representative side chains, not reaction mechanisms or protonation at a fixed pH.

use std::fmt::Write;
use std::path::PathBuf;

const INK: &str = "#17131f";
const CHANGE: &str = "#713caf";

#[derive(Clone, Copy)]
struct Atom {
    x: f64,
    y: f64,
    width: f64,
}

struct Diagram {
    svg: String,
}

impl Diagram {
    fn new(unimod: u16, digestion: bool) -> Self {
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"480\" height=\"480\" viewBox=\"0 0 480 480\" \
             font-family=\"Arial, Helvetica, sans-serif\" font-size=\"27\" fill=\"{INK}\">\n\
             <title>Before and after chemical structures</title>\n\
             <desc>Connectivity schematic. R is the unchanged remainder of the peptide. \
             Purple highlights the featured group; protonation is not shown at a fixed pH.</desc>\n\
             <!-- Chemistry: https://www.unimod.org/modifications_view.php?editid1={unimod} -->\n\
             <rect width=\"480\" height=\"480\" rx=\"16\" fill=\"#fff\" />\n"
        );
        let before = if digestion { "Before digestion" } else { "Before" };
        let after = if digestion { "After digestion" } else { "After" };
        writeln!(svg, "<text x=\"24\" y=\"30\" font-size=\"21\">{before}</text>").unwrap();
        writeln!(svg, "<text x=\"24\" y=\"253\" font-size=\"21\">{after}</text>").unwrap();
        writeln!(
            svg,
            "<path d=\"M240 214V238M231 231L240 240L249 231\" fill=\"none\" stroke=\"{INK}\" \
             stroke-width=\"3\" stroke-linecap=\"round\" stroke-linejoin=\"round\" />"
        )
        .unwrap();
        Self { svg }
    }

    fn atom(&mut self, x: f64, y: f64, label: &str, changed: bool) -> Atom {
        let color = if changed { CHANGE } else { INK };
        writeln!(
            self.svg,
            "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" dominant-baseline=\"central\" fill=\"{color}\">{label}</text>"
        )
        .unwrap();
        let width = label
            .chars()
            .map(|c| match c {
                '₁'..='₉' | '⁺' | '⁻' | '′' => 10.0,
                '(' | ')' => 8.0,
                _ => 19.0,
            })
            .sum();
        Atom { x, y, width }
    }

    fn point(x: f64, y: f64) -> Atom {
        Atom { x, y, width: 0.0 }
    }

    fn line(&mut self, a: (f64, f64), b: (f64, f64), changed: bool) {
        let color = if changed { CHANGE } else { INK };
        writeln!(
            self.svg,
            "<path d=\"M{:.1} {:.1}L{:.1} {:.1}\" fill=\"none\" stroke=\"{color}\" \
             stroke-width=\"2.8\" stroke-linecap=\"round\" />",
            a.0, a.1, b.0, b.1
        )
        .unwrap();
    }

    fn bond(&mut self, a: Atom, b: Atom, double: bool, changed: bool) {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let length = dx.hypot(dy);
        let (ux, uy) = (dx / length, dy / length);
        let trim = |atom: Atom| {
            if atom.width == 0.0 {
                0.0
            } else {
                ((atom.width / 2.0) / ux.abs().max(0.001)).min(12.0 / uy.abs().max(0.001)) + 4.0
            }
        };
        let start = (a.x + ux * trim(a), a.y + uy * trim(a));
        let end = (b.x - ux * trim(b), b.y - uy * trim(b));
        if double {
            for offset in [-2.8, 2.8] {
                self.line(
                    (start.0 - uy * offset, start.1 + ux * offset),
                    (end.0 - uy * offset, end.1 + ux * offset),
                    changed,
                );
            }
        } else {
            self.line(start, end, changed);
        }
    }

    fn chain(&mut self, y: f64, atoms: &[(f64, &str, bool)]) -> Vec<Atom> {
        let nodes: Vec<_> = atoms.iter().map(|(x, s, c)| self.atom(*x, y, s, *c)).collect();
        for (pair, atom) in nodes.windows(2).zip(atoms.iter().skip(1)) {
            self.bond(pair[0], pair[1], false, atom.2);
        }
        nodes
    }

    fn double_oxygen(&mut self, center: Atom, y: f64, changed: bool) {
        let oxygen = self.atom(center.x, y, "O", changed);
        self.bond(center, oxygen, true, changed);
    }

    fn finish(mut self, tag: bool) -> String {
        let footer = if tag { 448 } else { 460 };
        writeln!(
            self.svg,
            "<text x=\"240\" y=\"{footer}\" text-anchor=\"middle\" font-size=\"17\">R = unchanged part of the peptide</text>"
        )
        .unwrap();
        if tag {
            self.svg.push_str(
                "<text x=\"240\" y=\"473\" text-anchor=\"middle\" font-size=\"16\">R′ = remainder of the attached protein</text>\n",
            );
        }
        self.svg.push_str("</svg>\n");
        self.svg
    }
}

fn lysine(d: &mut Diagram, y: f64, nitrogen: &str) -> Atom {
    d.chain(
        y,
        &[
            (35.0, "R", false),
            (133.0, "(CH₂)₄", false),
            (245.0, nitrogen, false),
        ],
    )[2]
}

fn acyl(d: &mut Diagram, succinyl: bool) {
    lysine(d, 110.0, "NH₂");
    let n = lysine(d, 325.0, "NH");
    let c = d.atom(330.0, 325.0, "C", true);
    d.bond(n, c, false, true);
    d.double_oxygen(c, 270.0, true);
    if succinyl {
        let atoms = [
            d.atom(425.0, 325.0, "CH₂", true),
            d.atom(425.0, 385.0, "CH₂", true),
            d.atom(330.0, 385.0, "C", true),
            d.atom(235.0, 385.0, "OH", true),
        ];
        d.bond(c, atoms[0], false, true);
        for pair in atoms.windows(2) {
            d.bond(pair[0], pair[1], false, true);
        }
        d.double_oxygen(atoms[2], 428.0, true);
    } else {
        let methyl = d.atom(425.0, 325.0, "CH₃", true);
        d.bond(c, methyl, false, true);
    }
}

fn methyl(d: &mut Diagram, count: u8) {
    lysine(d, 110.0, "NH₂");
    let label = match count {
        1 => "NH",
        2 => "N",
        _ => "N⁺",
    };
    let n = lysine(d, 330.0, label);
    let right = d.atom(365.0, 330.0, "CH₃", true);
    d.bond(n, right, false, true);
    if count >= 2 {
        let above = d.atom(245.0, 275.0, "CH₃", true);
        d.bond(n, above, false, true);
    }
    if count == 3 {
        let below = d.atom(245.0, 400.0, "CH₃", true);
        d.bond(n, below, false, true);
    }
}

fn aromatic(d: &mut Diagram, y: f64) -> Atom {
    let root = d.chain(y, &[(32.0, "R", false), (100.0, "CH₂", false)]);
    let points: Vec<_> = (0..6)
        .map(|i| {
            let angle = (180.0 + i as f64 * 60.0).to_radians();
            (200.0 + 45.0 * angle.cos(), y + 45.0 * angle.sin())
        })
        .collect();
    d.bond(root[1], Diagram::point(points[0].0, points[0].1), false, false);
    for i in 0..6 {
        d.line(points[i], points[(i + 1) % 6], false);
    }
    for i in [0, 2, 4] {
        let a = points[i];
        let b = points[(i + 1) % 6];
        let inward = |p: (f64, f64)| (200.0 + (p.0 - 200.0) * 0.76, y + (p.1 - y) * 0.76);
        d.line(inward(a), inward(b), false);
    }
    Diagram::point(points[3].0, points[3].1)
}

fn pyroglutamate(d: &mut Diagram) {
    let chain = d.chain(
        100.0,
        &[
            (35.0, "R", false),
            (125.0, "CH", false),
            (225.0, "CH₂", false),
            (325.0, "CH₂", false),
        ],
    );
    let amine = d.atom(125.0, 50.0, "NH₂", false);
    d.bond(chain[1], amine, false, false);
    let carbonyl = d.atom(385.0, 160.0, "C", false);
    d.bond(chain[3], carbonyl, false, false);
    d.double_oxygen(carbonyl, 205.0, false);
    let leaving = d.atom(445.0, 160.0, "NH₂", true);
    d.bond(carbonyl, leaving, false, true);

    let alpha = d.atom(145.0, 295.0, "CH", false);
    let root = d.atom(55.0, 295.0, "R", false);
    d.bond(root, alpha, false, false);
    let nodes = [
        alpha,
        d.atom(235.0, 280.0, "CH₂", false),
        d.atom(280.0, 350.0, "CH₂", false),
        d.atom(205.0, 390.0, "C", false),
        d.atom(125.0, 375.0, "NH", false),
    ];
    for pair in nodes.windows(2) {
        d.bond(pair[0], pair[1], false, pair[0].x == 205.0);
    }
    d.bond(nodes[4], alpha, false, false);
    let oxygen = d.atom(245.0, 430.0, "O", false);
    d.bond(nodes[3], oxygen, true, false);
    d.atom(385.0, 400.0, "+ NH₃", true);
}

fn diglycine(d: &mut Diagram) {
    for (y, changed) in [(80.0, false), (300.0, true)] {
        let n = lysine(d, y, "NH");
        let c1 = d.atom(335.0, y, "C", changed);
        d.bond(n, c1, false, changed);
        d.double_oxygen(c1, y - 40.0, changed);
        let nodes = [
            c1,
            d.atom(425.0, y, "CH₂", changed),
            d.atom(425.0, y + 58.0, "NH", changed),
            d.atom(335.0, y + 58.0, "C", changed),
            d.atom(245.0, y + 58.0, "CH₂", changed),
            d.atom(245.0, y + 112.0, if changed { "NH₂" } else { "NH" }, changed),
        ];
        for pair in nodes.windows(2) {
            d.bond(pair[0], pair[1], false, changed);
        }
        d.double_oxygen(nodes[3], y + 104.0, changed);
        if !changed {
            let tag = d.atom(125.0, y + 112.0, "R′", true);
            d.bond(nodes[5], tag, false, true);
        }
    }
}

fn render(id: &str, unimod: u16) -> String {
    let mut d = Diagram::new(unimod, id == "gg");
    match id {
        "phospho" => {
            d.chain(
                110.0,
                &[(35.0, "R", false), (120.0, "CH₂", false), (210.0, "OH", false)],
            );
            let chain = d.chain(
                330.0,
                &[
                    (35.0, "R", false),
                    (120.0, "CH₂", false),
                    (205.0, "O", false),
                    (295.0, "P", true),
                    (405.0, "OH", true),
                ],
            );
            d.double_oxygen(chain[3], 275.0, true);
            let hydroxyl = d.atom(295.0, 400.0, "OH", true);
            d.bond(chain[3], hydroxyl, false, true);
        }
        "acetyl" => acyl(&mut d, false),
        "methyl" => methyl(&mut d, 1),
        "dimethyl" => methyl(&mut d, 2),
        "trimethyl" => methyl(&mut d, 3),
        "oxidation" => {
            for y in [110.0, 330.0] {
                let chain = d.chain(
                    y,
                    &[
                        (35.0, "R", false),
                        (135.0, "(CH₂)₂", false),
                        (255.0, "S", false),
                        (365.0, "CH₃", false),
                    ],
                );
                if y > 200.0 {
                    d.double_oxygen(chain[2], 275.0, true);
                }
            }
        }
        "deamidation" => {
            for y in [110.0, 330.0] {
                let after = y > 200.0;
                let chain = d.chain(
                    y,
                    &[
                        (35.0, "R", false),
                        (125.0, "CH₂", false),
                        (235.0, "C", false),
                        (365.0, if after { "OH" } else { "NH₂" }, true),
                    ],
                );
                d.double_oxygen(chain[2], y - 55.0, false);
            }
        }
        "carbamidomethyl" => {
            d.chain(
                110.0,
                &[(35.0, "R", false), (115.0, "CH₂", false), (205.0, "SH", false)],
            );
            let chain = d.chain(
                330.0,
                &[
                    (35.0, "R", false),
                    (110.0, "CH₂", false),
                    (190.0, "S", false),
                    (270.0, "CH₂", true),
                    (355.0, "C", true),
                    (435.0, "NH₂", true),
                ],
            );
            d.double_oxygen(chain[4], 275.0, true);
        }
        "gg" => diglycine(&mut d),
        "citrullination" => {
            for y in [110.0, 330.0] {
                let after = y > 200.0;
                let chain = d.chain(
                    y,
                    &[
                        (35.0, "R", false),
                        (130.0, "(CH₂)₃", false),
                        (245.0, "NH", false),
                        (330.0, "C", false),
                        (425.0, "NH₂", false),
                    ],
                );
                let group = d.atom(330.0, y - 55.0, if after { "O" } else { "NH" }, true);
                d.bond(chain[3], group, true, true);
            }
        }
        "pyroglu" => pyroglutamate(&mut d),
        "sulfo" => {
            let ring = aromatic(&mut d, 110.0);
            let hydroxyl = d.atom(325.0, 110.0, "OH", false);
            d.bond(ring, hydroxyl, false, false);
            let ring = aromatic(&mut d, 330.0);
            let chain = d.chain(
                330.0,
                &[(285.0, "O", false), (355.0, "S", true), (435.0, "OH", true)],
            );
            d.bond(ring, chain[0], false, false);
            d.double_oxygen(chain[1], 275.0, true);
            d.double_oxygen(chain[1], 400.0, true);
        }
        "succinyl" => acyl(&mut d, true),
        "nitro" => {
            for y in [110.0, 355.0] {
                let ring = aromatic(&mut d, y);
                let hydroxyl = d.atom(325.0, y, "OH", false);
                d.bond(ring, hydroxyl, false, false);
                if y > 200.0 {
                    let site = Diagram::point(222.5, y - 38.97);
                    let n = d.atom(310.0, 285.0, "N⁺", true);
                    let double_o = d.atom(365.0, 250.0, "O", true);
                    let single_o = d.atom(405.0, 310.0, "O⁻", true);
                    d.bond(site, n, false, true);
                    d.bond(n, double_o, true, true);
                    d.bond(n, single_o, false, true);
                }
            }
        }
        _ => unreachable!("all diagram IDs are declared below"),
    }
    d.finish(id == "gg")
}

fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    anyhow::ensure!(args.len() <= 1, "usage: render_ptm_diagrams [output-directory]");
    let output = args
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/public/study/ptms"));
    std::fs::create_dir_all(&output)?;
    for (id, unimod) in [
        ("phospho", 21),
        ("acetyl", 1),
        ("methyl", 34),
        ("dimethyl", 36),
        ("trimethyl", 37),
        ("oxidation", 35),
        ("deamidation", 7),
        ("carbamidomethyl", 4),
        ("gg", 121),
        ("citrullination", 7),
        ("pyroglu", 28),
        ("sulfo", 40),
        ("succinyl", 64),
        ("nitro", 354),
    ] {
        std::fs::write(output.join(format!("{id}.svg")), render(id, unimod))?;
    }
    println!("Rendered 14 PTM diagrams in {}", output.display());
    Ok(())
}
