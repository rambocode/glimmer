#!/usr/bin/env python3
"""从 logo.png 的几何生成输入法菜单图标 menu-icon.pdf：16×16pt 纯黑矢量图，斜线镂空。

系统只把「只有黑色 + 透明」的图当模板图（菜单高亮时反白、深色模式自动变色），
所以这里不用黑底白线的位图，而是圆角方块填黑、三条圆头斜线用 even-odd 挖空。
几何常数是从 logo.png（1254×1254）量出来的像素坐标，改了 logo 要重新量（量法见 README）。
"""
from pathlib import Path

# logo.png 里圆角方块的左上角、边长与圆角半径（像素）
SQ_X, SQ_Y, SQ_W, RADIUS = 242, 243, 769, 175
# 三条斜线：中心线起止点与线宽（像素）
STROKES = [
    ((717, 435), (414, 819), 78),
    ((821, 575), (687, 701), 72),
    ((864, 713), (769, 793), 68),
]
SIZE = 16.0  # 画布边长（pt），与鼠须管等菜单图标同高
K = 0.5523  # 四分之一圆的贝塞尔控制点系数


def pt(p):
    """像素坐标 → PDF 坐标（pt，y 轴向上）。"""
    s = SIZE / SQ_W
    return ((p[0] - SQ_X) * s, SIZE - (p[1] - SQ_Y) * s)


def rounded_rect(r):
    """整幅画布的圆角方块路径。"""
    w = SIZE
    return " ".join(
        [
            f"{r:.3f} 0 m",
            f"{w - r:.3f} 0 l",
            f"{w - r + K * r:.3f} 0 {w:.3f} {r - K * r:.3f} {w:.3f} {r:.3f} c",
            f"{w:.3f} {w - r:.3f} l",
            f"{w:.3f} {w - r + K * r:.3f} {w - r + K * r:.3f} {w:.3f} {w - r:.3f} {w:.3f} c",
            f"{r:.3f} {w:.3f} l",
            f"{r - K * r:.3f} {w:.3f} 0 {w - r + K * r:.3f} 0 {w - r:.3f} c",
            f"0 {r:.3f} l",
            f"0 {r - K * r:.3f} {r - K * r:.3f} 0 {r:.3f} 0 c",
            "h",
        ]
    )


def arc(center, start_dir, end_dir, r):
    """以 center 为圆心、从 start_dir 转到 end_dir（相差 90°）的四分之一圆弧。"""
    cx, cy = center
    sx, sy = start_dir
    ex, ey = end_dir
    p1 = (cx + r * sx + K * r * ex, cy + r * sy + K * r * ey)
    p2 = (cx + r * ex + K * r * sx, cy + r * ey + K * r * sy)
    return f"{p1[0]:.3f} {p1[1]:.3f} {p2[0]:.3f} {p2[1]:.3f} {cx + r * ex:.3f} {cy + r * ey:.3f} c"


def capsule(a, b, width):
    """圆头线段的外轮廓（胶囊形）：两条直边加两端各两段四分之一圆弧。"""
    (ax, ay), (bx, by) = pt(a), pt(b)
    h = width * SIZE / SQ_W / 2
    dx, dy = bx - ax, by - ay
    length = (dx * dx + dy * dy) ** 0.5
    d = (dx / length, dy / length)
    n = (-d[1], d[0])
    neg_d = (-d[0], -d[1])
    neg_n = (-n[0], -n[1])
    return " ".join(
        [
            f"{ax + n[0] * h:.3f} {ay + n[1] * h:.3f} m",
            f"{bx + n[0] * h:.3f} {by + n[1] * h:.3f} l",
            arc((bx, by), n, d, h),
            arc((bx, by), d, neg_n, h),
            f"{ax - n[0] * h:.3f} {ay - n[1] * h:.3f} l",
            arc((ax, ay), neg_n, neg_d, h),
            arc((ax, ay), neg_d, n, h),
            "h",
        ]
    )


def build_pdf(content: str) -> bytes:
    """把一段内容流包成最小的单页 PDF（手写 xref，不依赖第三方库）。"""
    objs = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {SIZE:g} {SIZE:g}] /Contents 4 0 R >>".encode(),
        b"<< /Length %d >>\nstream\n%s\nendstream" % (len(content), content.encode()),
    ]
    out = bytearray(b"%PDF-1.4\n")
    offsets = []
    for i, body in enumerate(objs, 1):
        offsets.append(len(out))
        out += b"%d 0 obj\n%s\nendobj\n" % (i, body)
    xref = len(out)
    out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1)
    for off in offsets:
        out += b"%010d 00000 n \n" % off
    out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, xref)
    return bytes(out)


def main():
    paths = [rounded_rect(RADIUS * SIZE / SQ_W)] + [capsule(a, b, w) for a, b, w in STROKES]
    content = "0 0 0 rg\n" + "\n".join(paths) + "\nf*\n"
    out = Path(__file__).with_name("menu-icon.pdf")
    out.write_bytes(build_pdf(content))
    print(f"已生成 {out}")


if __name__ == "__main__":
    main()
