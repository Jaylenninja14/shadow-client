// Generates the menu's white 64x64 module icons into the mod's resources.
// Run: java tools/IconGen.java src/main/resources/assets/shadowmenu/textures/gui/icon
import java.awt.*;
import java.awt.geom.*;
import java.awt.image.BufferedImage;
import java.io.File;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.function.Consumer;
import javax.imageio.ImageIO;

public class IconGen {
    static final int S = 64;

    public static void main(String[] args) throws Exception {
        File out = new File(args[0]);
        out.mkdirs();
        Map<String, Consumer<Graphics2D>> icons = new LinkedHashMap<>();
        icons.put("fps", g -> { // speedometer
            g.draw(new Arc2D.Double(8, 12, 48, 48, 0, 180, Arc2D.OPEN));
            for (int a = 0; a <= 180; a += 45) {
                double r = Math.toRadians(a);
                g.draw(new Line2D.Double(32 + 24 * Math.cos(r), 36 - 24 * Math.sin(r), 32 + 18 * Math.cos(r), 36 - 18 * Math.sin(r)));
            }
            g.draw(new Line2D.Double(32, 36, 46, 22));
            g.fill(new Ellipse2D.Double(28, 32, 8, 8));
            text(g, "FPS", 50, 14);
        });
        icons.put("cps", g -> { // mouse with click waves
            g.draw(new RoundRectangle2D.Double(20, 14, 24, 40, 22, 22));
            g.draw(new Line2D.Double(32, 14, 32, 28));
            g.fill(new RoundRectangle2D.Double(21, 15, 11, 13, 10, 10));
            g.draw(new Arc2D.Double(4, 6, 16, 16, 90, 90, Arc2D.OPEN));
            g.draw(new Arc2D.Double(44, 6, 16, 16, 0, 90, Arc2D.OPEN));
        });
        icons.put("keystrokes", g -> { // W + A S D keys
            key(g, 23, 8, "W");
            key(g, 4, 28, "A");
            key(g, 23, 28, "S");
            key(g, 42, 28, "D");
            g.fill(new RoundRectangle2D.Double(4, 48, 56, 8, 4, 4));
        });
        icons.put("ping", g -> { // signal bars
            for (int i = 0; i < 4; i++) g.fill(new RoundRectangle2D.Double(8 + i * 13, 44 - i * 10, 9, 12 + i * 10, 3, 3));
        });
        icons.put("armor", g -> { // chestplate
            Path2D p = new Path2D.Double();
            p.moveTo(14, 10); p.lineTo(24, 10); p.quadTo(32, 18, 40, 10); p.lineTo(50, 10);
            p.lineTo(56, 22); p.lineTo(48, 26); p.lineTo(48, 54); p.lineTo(16, 54); p.lineTo(16, 26); p.lineTo(8, 22); p.closePath();
            g.fill(p);
        });
        icons.put("potion", g -> { // bottle
            g.fill(new Rectangle2D.Double(26, 6, 12, 6));
            g.draw(new Line2D.Double(28, 12, 28, 22));
            g.draw(new Line2D.Double(36, 12, 36, 22));
            g.draw(new Ellipse2D.Double(12, 20, 40, 38));
            g.fill(new Arc2D.Double(15, 23, 34, 32, 200, 140, Arc2D.CHORD));
        });
        icons.put("coords", g -> { // XYZ axes
            g.draw(new Line2D.Double(22, 46, 22, 10));
            g.draw(new Line2D.Double(22, 46, 56, 46));
            g.draw(new Line2D.Double(22, 46, 6, 60));
            text(g, "XYZ", 44, 26);
        });
        icons.put("compass", g -> {
            g.draw(new Ellipse2D.Double(8, 8, 48, 48));
            Path2D n = new Path2D.Double();
            n.moveTo(32, 14); n.lineTo(38, 32); n.lineTo(26, 32); n.closePath();
            g.fill(n);
            Path2D s = new Path2D.Double();
            s.moveTo(32, 50); s.lineTo(38, 32); s.lineTo(26, 32); s.closePath();
            g.draw(s);
        });
        icons.put("clock", g -> {
            g.draw(new Ellipse2D.Double(8, 8, 48, 48));
            g.draw(new Line2D.Double(32, 32, 32, 16));
            g.draw(new Line2D.Double(32, 32, 44, 38));
        });
        icons.put("speed", g -> { // lightning bolt
            Path2D p = new Path2D.Double();
            p.moveTo(36, 4); p.lineTo(14, 36); p.lineTo(30, 36); p.lineTo(26, 60); p.lineTo(50, 26); p.lineTo(34, 26); p.closePath();
            g.fill(p);
        });
        icons.put("reach", g -> { // sword + arrow distance
            g.draw(new Line2D.Double(10, 54, 40, 24));
            g.draw(new Line2D.Double(16, 40, 24, 48));
            g.draw(new Line2D.Double(40, 24, 50, 14));
            g.draw(new Line2D.Double(44, 52, 58, 52));
            g.draw(new Line2D.Double(52, 46, 58, 52));
            g.draw(new Line2D.Double(52, 58, 58, 52));
        });
        icons.put("combo", g -> text(g, "x3", 32, 44, 34f));
        icons.put("sprint", g -> { // running chevrons
            for (int i = 0; i < 3; i++) {
                int x = 10 + i * 16;
                g.draw(new Line2D.Double(x, 16, x + 14, 32));
                g.draw(new Line2D.Double(x + 14, 32, x, 48));
            }
        });
        icons.put("pack", g -> { // stacked squares
            g.draw(new Rectangle2D.Double(8, 20, 30, 30));
            g.draw(new Rectangle2D.Double(17, 14, 30, 30));
            g.fill(new Rectangle2D.Double(26, 8, 30, 30));
        });
        icons.put("motionblur", g -> { // dot burst like the reference
            for (int y = -3; y <= 3; y++) for (int x = -3; x <= 3; x++) {
                double d = Math.hypot(x, y);
                if (d > 3.3) continue;
                double r = 6.5 - d * 1.5;
                g.fill(new Ellipse2D.Double(32 + x * 8 - r / 2, 32 + y * 8 - r / 2, r, r));
            }
        });
        icons.put("freelook", g -> { // eye in corner brackets
            Path2D eye = new Path2D.Double();
            eye.moveTo(10, 32); eye.quadTo(32, 8, 54, 32); eye.quadTo(32, 56, 10, 32); eye.closePath();
            g.fill(eye);
            g.setColor(new Color(0, 0, 0, 0));
            g.setComposite(AlphaComposite.Clear);
            g.fill(new Ellipse2D.Double(23, 23, 18, 18));
            g.setComposite(AlphaComposite.SrcOver);
            g.setColor(Color.WHITE);
            g.fill(new Ellipse2D.Double(27, 27, 10, 10));
            corner(g);
        });
        icons.put("zoom", g -> { // magnifier
            g.draw(new Ellipse2D.Double(10, 8, 32, 32));
            g.setStroke(new BasicStroke(8, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
            g.draw(new Line2D.Double(40, 38, 54, 52));
        });
        icons.put("fullbright", g -> { // sun + slider
            g.fill(new Ellipse2D.Double(22, 12, 20, 20));
            for (int a = 0; a < 360; a += 45) {
                double r = Math.toRadians(a);
                g.draw(new Line2D.Double(32 + 15 * Math.cos(r), 22 + 15 * Math.sin(r), 32 + 21 * Math.cos(r), 22 + 21 * Math.sin(r)));
            }
            g.draw(new Line2D.Double(12, 52, 52, 52));
            g.fill(new Ellipse2D.Double(36, 47, 10, 10));
        });
        icons.put("outline", g -> { // cube outline
            Path2D p = new Path2D.Double();
            p.moveTo(32, 6); p.lineTo(56, 18); p.lineTo(56, 46); p.lineTo(32, 58); p.lineTo(8, 46); p.lineTo(8, 18); p.closePath();
            p.moveTo(8, 18); p.lineTo(32, 30); p.lineTo(56, 18);
            p.moveTo(32, 30); p.lineTo(32, 58);
            g.draw(p);
        });
        icons.put("time", g -> { // moon + star
            Area moon = new Area(new Ellipse2D.Double(10, 10, 42, 42));
            moon.subtract(new Area(new Ellipse2D.Double(22, 4, 40, 40)));
            g.fill(moon);
            g.fill(new Ellipse2D.Double(46, 12, 6, 6));
        });
        icons.put("hitcolor", g -> { // heart with hit lines
            Path2D h = new Path2D.Double();
            h.moveTo(32, 54); h.curveTo(4, 34, 10, 10, 32, 22); h.curveTo(54, 10, 60, 34, 32, 54); h.closePath();
            g.fill(h);
        });
        icons.put("hurtcam", g -> { // tilted camera
            AffineTransform t = g.getTransform();
            g.rotate(Math.toRadians(-12), 32, 32);
            g.draw(new RoundRectangle2D.Double(8, 20, 48, 32, 8, 8));
            g.draw(new Ellipse2D.Double(22, 26, 20, 20));
            g.fill(new Rectangle2D.Double(22, 14, 14, 6));
            g.setTransform(t);
        });
        icons.put("fire", g -> {
            Path2D p = new Path2D.Double();
            p.moveTo(32, 58); p.curveTo(10, 58, 8, 34, 24, 20); p.curveTo(24, 32, 32, 32, 30, 6);
            p.curveTo(48, 18, 58, 40, 44, 54); p.curveTo(40, 58, 36, 58, 32, 58); p.closePath();
            g.fill(p);
            g.setColor(Color.BLACK);
            g.fill(new Ellipse2D.Double(26, 40, 12, 14));
            g.setColor(Color.WHITE);
            text(g, "LOW", 32, 64, 11f);
        });
        icons.put("shield", g -> {
            Path2D p = new Path2D.Double();
            p.moveTo(32, 6); p.lineTo(54, 14); p.lineTo(52, 36); p.curveTo(48, 50, 40, 54, 32, 60); p.curveTo(24, 54, 16, 50, 12, 36); p.lineTo(10, 14); p.closePath();
            g.draw(p);
            g.draw(new Line2D.Double(32, 10, 32, 54));
        });
        icons.put("viewbob", g -> { // wavy line
            Path2D p = new Path2D.Double();
            p.moveTo(6, 32);
            for (int x = 6; x <= 58; x += 2) p.lineTo(x, 32 + 10 * Math.sin((x - 6) / 52.0 * Math.PI * 2));
            g.draw(p);
        });
        icons.put("fov", g -> { // field of view cone
            g.draw(new Line2D.Double(10, 32, 54, 10));
            g.draw(new Line2D.Double(10, 32, 54, 54));
            g.draw(new Arc2D.Double(-14, 4, 72, 56, -30, 60, Arc2D.OPEN));
            g.fill(new Ellipse2D.Double(5, 27, 10, 10));
        });
        icons.put("vignette", g -> { // frame with dark edges
            g.draw(new RoundRectangle2D.Double(6, 12, 52, 40, 10, 10));
            g.draw(new Ellipse2D.Double(18, 20, 28, 24));
        });
        icons.put("rain", g -> { // cloud with slash
            Area c = new Area(new Ellipse2D.Double(10, 16, 22, 22));
            c.add(new Area(new Ellipse2D.Double(24, 10, 28, 28)));
            c.add(new Area(new RoundRectangle2D.Double(10, 26, 46, 16, 14, 14)));
            g.fill(c);
            for (int i = 0; i < 3; i++) g.draw(new Line2D.Double(18 + i * 12, 48, 14 + i * 12, 58));
        });
        icons.put("nametag", g -> {
            g.draw(new RoundRectangle2D.Double(6, 20, 52, 24, 8, 8));
            g.fill(new RoundRectangle2D.Double(14, 28, 22, 8, 4, 4));
            g.fill(new Ellipse2D.Double(42, 28, 8, 8));
        });
        icons.put("tooltip", g -> { // speech box with scroll arrows
            g.draw(new RoundRectangle2D.Double(6, 8, 40, 30, 8, 8));
            g.draw(new Line2D.Double(14, 18, 38, 18));
            g.draw(new Line2D.Double(14, 28, 32, 28));
            g.draw(new Line2D.Double(54, 34, 54, 58));
            g.draw(new Line2D.Double(48, 52, 54, 58));
            g.draw(new Line2D.Double(60, 52, 54, 58));
        });
        icons.put("crosshair", g -> {
            g.draw(new Line2D.Double(32, 8, 32, 24));
            g.draw(new Line2D.Double(32, 40, 32, 56));
            g.draw(new Line2D.Double(8, 32, 24, 32));
            g.draw(new Line2D.Double(40, 32, 56, 32));
            g.fill(new Ellipse2D.Double(29, 29, 6, 6));
        });
        icons.put("hotbar", g -> {
            for (int i = 0; i < 4; i++) g.draw(new Rectangle2D.Double(4 + i * 14.5, 22, 12, 20));
            g.setStroke(new BasicStroke(6));
            g.draw(new Rectangle2D.Double(3, 20, 15, 24));
        });
        icons.put("scoreboard", g -> {
            g.draw(new Rectangle2D.Double(10, 8, 44, 48));
            for (int i = 0; i < 4; i++) g.draw(new Line2D.Double(16, 18 + i * 10, 48, 18 + i * 10));
        });
        icons.put("bossbar", g -> {
            text(g, "BOSS", 32, 28, 18f);
            g.draw(new RoundRectangle2D.Double(6, 38, 52, 10, 6, 6));
            g.fill(new RoundRectangle2D.Double(6, 38, 34, 10, 6, 6));
        });
        icons.put("memory", g -> { // RAM stick
            g.draw(new Rectangle2D.Double(6, 18, 52, 24));
            for (int i = 0; i < 4; i++) g.fill(new Rectangle2D.Double(11 + i * 12, 23, 8, 14));
            for (int i = 0; i < 7; i++) g.fill(new Rectangle2D.Double(9 + i * 7, 44, 4, 8));
        });
        icons.put("arrow", g -> {
            g.draw(new Line2D.Double(10, 54, 50, 14));
            g.draw(new Line2D.Double(50, 14, 38, 16));
            g.draw(new Line2D.Double(50, 14, 48, 26));
            g.draw(new Line2D.Double(14, 50, 8, 44));
            g.draw(new Line2D.Double(14, 50, 20, 56));
        });
        icons.put("chat", g -> {
            Path2D p = new Path2D.Double();
            p.moveTo(8, 10); p.lineTo(56, 10); p.lineTo(56, 42); p.lineTo(24, 42); p.lineTo(12, 54); p.lineTo(14, 42); p.lineTo(8, 42); p.closePath();
            g.draw(p);
        });
        icons.put("player", g -> {
            g.fill(new Rectangle2D.Double(22, 6, 20, 20));
            g.fill(new Rectangle2D.Double(18, 28, 28, 20));
            g.fill(new Rectangle2D.Double(20, 48, 10, 12));
            g.fill(new Rectangle2D.Double(34, 48, 10, 12));
        });
        icons.put("server", g -> {
            for (int i = 0; i < 3; i++) {
                g.draw(new RoundRectangle2D.Double(10, 6 + i * 18, 44, 14, 4, 4));
                g.fill(new Ellipse2D.Double(44, 10 + i * 18, 5, 5));
            }
        });
        icons.put("generic", g -> { // puzzle piece
            Area a = new Area(new Rectangle2D.Double(12, 18, 36, 36));
            a.add(new Area(new Ellipse2D.Double(24, 8, 14, 14)));
            a.add(new Area(new Ellipse2D.Double(42, 30, 14, 14)));
            g.fill(a);
        });
        icons.put("gear", g -> {
            Area a = new Area(new Ellipse2D.Double(14, 14, 36, 36));
            for (int i = 0; i < 8; i++) {
                AffineTransform t = AffineTransform.getRotateInstance(Math.toRadians(i * 45), 32, 32);
                a.add(new Area(t.createTransformedShape(new Rectangle2D.Double(27, 4, 10, 14))));
            }
            a.subtract(new Area(new Ellipse2D.Double(24, 24, 16, 16)));
            g.fill(a);
        });
        // ---- added in 1.1.0 (Performance, Hypixel and more)
        icons.put("graphics", g -> { // monitor with a sparkle
            g.draw(new RoundRectangle2D.Double(6, 10, 52, 34, 6, 6));
            g.draw(new Line2D.Double(32, 44, 32, 54));
            g.draw(new Line2D.Double(20, 55, 44, 55));
            star(g, 32, 27, 10);
        });
        icons.put("distance", g -> { // mountains with an arrow to the horizon
            Path2D p = new Path2D.Double();
            p.moveTo(4, 52); p.lineTo(22, 26); p.lineTo(32, 38); p.lineTo(42, 22); p.lineTo(60, 52); p.closePath();
            g.draw(p);
            g.draw(new Line2D.Double(10, 14, 54, 14));
            g.draw(new Line2D.Double(54, 14, 47, 8));
            g.draw(new Line2D.Double(54, 14, 47, 20));
        });
        icons.put("chunk", g -> { // a 3x3 block of chunks
            for (int y = 0; y < 3; y++) for (int x = 0; x < 3; x++) {
                if (x == 1 && y == 1) g.fill(new Rectangle2D.Double(8 + x * 17, 8 + y * 17, 14, 14));
                else g.draw(new Rectangle2D.Double(8 + x * 17, 8 + y * 17, 14, 14));
            }
        });
        icons.put("cloud", g -> cloud(g, 0));
        icons.put("particles", g -> { // sparks
            g.fill(new Ellipse2D.Double(28, 28, 8, 8));
            for (int a = 0; a < 360; a += 45) {
                double r = Math.toRadians(a);
                double d = a % 90 == 0 ? 22 : 16;
                g.fill(new Ellipse2D.Double(32 + d * Math.cos(r) - 3, 32 + d * Math.sin(r) - 3, 6, 6));
            }
        });
        icons.put("shadow", g -> { // block with its shadow on the ground
            g.fill(new Rectangle2D.Double(20, 8, 24, 24));
            g.fill(new Ellipse2D.Double(10, 44, 44, 10));
        });
        icons.put("light", g -> { // light bulb
            g.draw(new Ellipse2D.Double(16, 6, 32, 32));
            g.draw(new Line2D.Double(26, 38, 26, 46));
            g.draw(new Line2D.Double(38, 38, 38, 46));
            g.draw(new Line2D.Double(24, 48, 40, 48));
            g.draw(new Line2D.Double(27, 55, 37, 55));
        });
        icons.put("blend", g -> { // two overlapping tiles
            g.draw(new RoundRectangle2D.Double(6, 6, 34, 34, 6, 6));
            g.fill(new RoundRectangle2D.Double(24, 24, 34, 34, 6, 6));
        });
        icons.put("leaves", g -> {
            Path2D p = new Path2D.Double();
            p.moveTo(10, 54); p.curveTo(8, 20, 30, 8, 56, 8); p.curveTo(56, 34, 44, 56, 10, 54); p.closePath();
            g.draw(p);
            g.draw(new Line2D.Double(10, 54, 44, 20));
        });
        icons.put("glass", g -> { // window pane with a glare
            g.draw(new Rectangle2D.Double(8, 8, 48, 48));
            g.draw(new Line2D.Double(32, 8, 32, 56));
            g.draw(new Line2D.Double(8, 32, 56, 32));
            g.draw(new Line2D.Double(14, 24, 24, 14));
        });
        icons.put("vsync", g -> { // two arrows in a circle
            g.draw(new Arc2D.Double(10, 10, 44, 44, 20, 150, Arc2D.OPEN));
            g.draw(new Arc2D.Double(10, 10, 44, 44, 200, 150, Arc2D.OPEN));
            g.draw(new Line2D.Double(13, 20, 13, 32)); g.draw(new Line2D.Double(13, 32, 24, 30));
            g.draw(new Line2D.Double(51, 44, 51, 32)); g.draw(new Line2D.Double(51, 32, 40, 34));
        });
        icons.put("fullscreen", IconGen::corner);
        icons.put("storm", g -> { // cloud with a bolt
            cloud(g, -8);
            Path2D b = new Path2D.Double();
            b.moveTo(34, 36); b.lineTo(26, 48); b.lineTo(33, 48); b.lineTo(28, 60); b.lineTo(42, 44); b.lineTo(35, 44); b.lineTo(40, 36); b.closePath();
            g.fill(b);
        });
        icons.put("glint", g -> {
            star(g, 26, 34, 20);
            star(g, 48, 14, 9);
        });
        icons.put("brightness", g -> { // sun
            g.fill(new Ellipse2D.Double(20, 20, 24, 24));
            for (int a = 0; a < 360; a += 45) {
                double r = Math.toRadians(a);
                g.draw(new Line2D.Double(32 + 18 * Math.cos(r), 32 + 18 * Math.sin(r), 32 + 27 * Math.cos(r), 32 + 27 * Math.sin(r)));
            }
        });
        icons.put("gui", g -> { // window with a layout
            g.draw(new RoundRectangle2D.Double(6, 8, 52, 48, 6, 6));
            g.draw(new Line2D.Double(6, 20, 58, 20));
            g.fill(new Rectangle2D.Double(14, 28, 14, 20));
            g.fill(new Rectangle2D.Double(34, 28, 16, 6));
            g.fill(new Rectangle2D.Double(34, 40, 16, 6));
        });
        icons.put("blur", g -> {
            for (int i = 0; i < 3; i++) {
                g.setComposite(AlphaComposite.getInstance(AlphaComposite.SRC_OVER, 0.35f + i * 0.3f));
                g.fill(new Ellipse2D.Double(8 + i * 8, 14 + i * 4, 32, 32));
            }
            g.setComposite(AlphaComposite.SrcOver);
        });
        icons.put("attack", g -> { // sword
            g.draw(new Line2D.Double(14, 50, 52, 12));
            g.draw(new Line2D.Double(52, 12, 54, 10));
            g.draw(new Line2D.Double(12, 38, 26, 52));
            g.draw(new Line2D.Double(8, 56, 16, 48));
        });
        icons.put("subtitle", g -> { // CC box
            g.draw(new RoundRectangle2D.Double(6, 14, 52, 36, 8, 8));
            text(g, "CC", 32, 40, 18f);
        });
        icons.put("save", g -> { // floppy disk
            Path2D p = new Path2D.Double();
            p.moveTo(8, 8); p.lineTo(46, 8); p.lineTo(56, 18); p.lineTo(56, 56); p.lineTo(8, 56); p.closePath();
            g.draw(p);
            g.fill(new Rectangle2D.Double(18, 12, 22, 12));
            g.draw(new Rectangle2D.Double(16, 36, 32, 20));
        });
        icons.put("discord", g -> { // game controller
            g.draw(new RoundRectangle2D.Double(4, 18, 56, 30, 26, 26));
            g.draw(new Line2D.Double(16, 33, 26, 33));
            g.draw(new Line2D.Double(21, 28, 21, 38));
            g.fill(new Ellipse2D.Double(40, 26, 6, 6));
            g.fill(new Ellipse2D.Double(46, 34, 6, 6));
        });
        icons.put("camera", g -> {
            g.draw(new RoundRectangle2D.Double(6, 18, 52, 36, 8, 8));
            g.draw(new Ellipse2D.Double(22, 26, 20, 20));
            g.fill(new Rectangle2D.Double(22, 10, 20, 8));
        });
        icons.put("tnt", g -> {
            g.draw(new Rectangle2D.Double(8, 12, 48, 44));
            g.fill(new Rectangle2D.Double(8, 26, 48, 16));
            g.setColor(new Color(0, 0, 0, 0));
            g.setComposite(AlphaComposite.Clear);
            text(g, "TNT", 32, 39, 14f);
            g.setComposite(AlphaComposite.SrcOver);
            g.setColor(Color.WHITE);
            g.draw(new Line2D.Double(32, 12, 36, 4));
        });
        icons.put("gg", g -> text(g, "GG", 32, 42, 28f));
        icons.put("tip", g -> { // coin
            g.draw(new Ellipse2D.Double(8, 8, 48, 48));
            text(g, "$", 32, 42, 26f);
        });
        icons.put("boop", g -> { // speech bubble with "!"
            g.draw(new RoundRectangle2D.Double(6, 8, 52, 36, 12, 12));
            Path2D t = new Path2D.Double();
            t.moveTo(18, 44); t.lineTo(14, 56); t.lineTo(28, 44);
            g.draw(t);
            text(g, "!", 32, 36, 22f);
        });
        icons.put("level", g -> { // star with a number
            star(g, 32, 32, 28);
            g.setComposite(AlphaComposite.Clear);
            text(g, "1", 32, 39, 18f);
            g.setComposite(AlphaComposite.SrcOver);
        });
        icons.put("nick", g -> { // mask
            Path2D p = new Path2D.Double();
            p.moveTo(6, 22); p.curveTo(20, 14, 44, 14, 58, 22); p.curveTo(58, 40, 44, 46, 32, 38); p.curveTo(20, 46, 6, 40, 6, 22); p.closePath();
            g.fill(p);
            g.setComposite(AlphaComposite.Clear);
            g.fill(new Ellipse2D.Double(14, 24, 12, 8));
            g.fill(new Ellipse2D.Double(38, 24, 12, 8));
            g.setComposite(AlphaComposite.SrcOver);
        });
        icons.put("bed", g -> {
            g.draw(new Line2D.Double(6, 14, 6, 54));
            g.draw(new Line2D.Double(58, 34, 58, 54));
            g.draw(new Line2D.Double(6, 46, 58, 46));
            g.fill(new RoundRectangle2D.Double(10, 26, 14, 12, 6, 6));
            g.fill(new RoundRectangle2D.Double(26, 30, 32, 14, 6, 6));
        });
        icons.put("beacon", g -> { // pyramid and beam
            g.fill(new Rectangle2D.Double(28, 4, 8, 30));
            Path2D p = new Path2D.Double();
            p.moveTo(6, 58); p.lineTo(20, 38); p.lineTo(44, 38); p.lineTo(58, 58); p.closePath();
            g.draw(p);
        });
        icons.put("boost", g -> { // rocket
            Path2D p = new Path2D.Double();
            p.moveTo(32, 4); p.curveTo(46, 16, 46, 32, 40, 44); p.lineTo(24, 44); p.curveTo(18, 32, 18, 16, 32, 4); p.closePath();
            g.draw(p);
            g.fill(new Ellipse2D.Double(27, 18, 10, 10));
            g.draw(new Line2D.Double(24, 36, 14, 48));
            g.draw(new Line2D.Double(40, 36, 50, 48));
            g.draw(new Line2D.Double(32, 50, 32, 60));
        });
        icons.put("exit", g -> { // door with an arrow
            g.draw(new Rectangle2D.Double(8, 6, 26, 52));
            g.draw(new Line2D.Double(28, 32, 58, 32));
            g.draw(new Line2D.Double(58, 32, 50, 24));
            g.draw(new Line2D.Double(58, 32, 50, 40));
        });
        icons.put("darkness", g -> { // crescent moon
            Area a = new Area(new Ellipse2D.Double(8, 8, 46, 46));
            a.subtract(new Area(new Ellipse2D.Double(22, 2, 40, 40)));
            g.fill(a);
        });
        icons.put("tilt", g -> { // tilted horizon
            g.draw(new Ellipse2D.Double(8, 8, 48, 48));
            g.draw(new Line2D.Double(12, 40, 52, 24));
            g.fill(new Ellipse2D.Double(28, 28, 8, 8));
        });
        icons.put("splash", g -> text(g, "Aa", 32, 42, 26f));
        icons.put("entity", g -> { // creeper face
            g.draw(new Rectangle2D.Double(8, 8, 48, 48));
            g.fill(new Rectangle2D.Double(16, 18, 10, 10));
            g.fill(new Rectangle2D.Double(38, 18, 10, 10));
            g.fill(new Rectangle2D.Double(26, 30, 12, 8));
            g.fill(new Rectangle2D.Double(20, 36, 8, 12));
            g.fill(new Rectangle2D.Double(36, 36, 8, 12));
        });
        // ---- added in 1.2.0 (PvP pack)
        icons.put("sound", g -> { // speaker with waves
            Path2D p = new Path2D.Double();
            p.moveTo(8, 24); p.lineTo(18, 24); p.lineTo(32, 12); p.lineTo(32, 52); p.lineTo(18, 40); p.lineTo(8, 40); p.closePath();
            g.fill(p);
            g.draw(new Arc2D.Double(30, 20, 16, 24, -50, 100, Arc2D.OPEN));
            g.draw(new Arc2D.Double(30, 12, 28, 40, -50, 100, Arc2D.OPEN));
        });
        icons.put("music", g -> { // two notes
            g.fill(new Ellipse2D.Double(8, 42, 16, 12));
            g.fill(new Ellipse2D.Double(38, 36, 16, 12));
            g.draw(new Line2D.Double(22, 48, 22, 12));
            g.draw(new Line2D.Double(52, 42, 52, 8));
            g.fill(new Polygon(new int[] {22, 52, 52, 22}, new int[] {10, 6, 16, 20}, 4));
        });
        icons.put("mouse", g -> { // mouse
            g.draw(new RoundRectangle2D.Double(18, 6, 28, 52, 26, 26));
            g.draw(new Line2D.Double(32, 6, 32, 24));
            g.draw(new Line2D.Double(18, 24, 46, 24));
        });
        icons.put("skin", g -> { // player outline with a cape
            g.fill(new Rectangle2D.Double(24, 4, 16, 16));
            g.draw(new Rectangle2D.Double(22, 22, 20, 22));
            g.draw(new Line2D.Double(14, 22, 14, 40));
            g.draw(new Line2D.Double(50, 22, 50, 40));
            g.draw(new Line2D.Double(27, 44, 27, 60));
            g.draw(new Line2D.Double(37, 44, 37, 60));
        });
        icons.put("hand", g -> { // open hand
            for (int i = 0; i < 4; i++) g.fill(new RoundRectangle2D.Double(16 + i * 9, 8 + (i == 0 || i == 3 ? 6 : 0), 7, 26, 7, 7));
            g.fill(new RoundRectangle2D.Double(14, 28, 36, 28, 14, 14));
            g.fill(new RoundRectangle2D.Double(4, 26, 16, 7, 7, 7));
        });
        icons.put("hitbox", g -> { // box around a figure
            g.draw(new Rectangle2D.Double(14, 4, 36, 56));
            g.fill(new Rectangle2D.Double(26, 10, 12, 12));
            g.fill(new Rectangle2D.Double(24, 24, 16, 18));
            g.fill(new Rectangle2D.Double(24, 42, 6, 14));
            g.fill(new Rectangle2D.Double(34, 42, 6, 14));
            g.draw(new Line2D.Double(10, 16, 54, 16));
        });
        icons.put("chunkborder", g -> { // grid with a highlighted border
            g.setStroke(new BasicStroke(2));
            for (int i = 1; i < 4; i++) {
                g.draw(new Line2D.Double(8 + i * 12, 8, 8 + i * 12, 56));
                g.draw(new Line2D.Double(8, 8 + i * 12, 56, 8 + i * 12));
            }
            g.setStroke(new BasicStroke(5, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
            g.draw(new Rectangle2D.Double(8, 8, 48, 48));
        });
        icons.put("pause", g -> {
            g.fill(new RoundRectangle2D.Double(16, 12, 12, 40, 4, 4));
            g.fill(new RoundRectangle2D.Double(36, 12, 12, 40, 4, 4));
        });
        icons.put("heart", g -> {
            Path2D h = new Path2D.Double();
            h.moveTo(32, 56); h.curveTo(2, 34, 8, 6, 32, 20); h.curveTo(56, 6, 62, 34, 32, 56); h.closePath();
            g.fill(h);
        });
        icons.put("heart_outline", g -> {
            Path2D h = new Path2D.Double();
            h.moveTo(32, 54); h.curveTo(6, 34, 10, 10, 32, 22); h.curveTo(54, 10, 58, 34, 32, 54); h.closePath();
            g.draw(h);
        });
        icons.put("grid", g -> {
            for (int y = 0; y < 2; y++) for (int x = 0; x < 2; x++) g.fill(new RoundRectangle2D.Double(10 + x * 24, 10 + y * 24, 20, 20, 6, 6));
        });
        for (var e : icons.entrySet()) {
            BufferedImage img = new BufferedImage(S, S, BufferedImage.TYPE_INT_ARGB);
            Graphics2D g = img.createGraphics();
            g.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON);
            g.setRenderingHint(RenderingHints.KEY_STROKE_CONTROL, RenderingHints.VALUE_STROKE_PURE);
            g.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, RenderingHints.VALUE_TEXT_ANTIALIAS_ON);
            g.setColor(Color.WHITE);
            g.setStroke(new BasicStroke(5, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
            e.getValue().accept(g);
            g.dispose();
            ImageIO.write(img, "png", new File(out, e.getKey() + ".png"));
        }
        System.out.println("wrote " + icons.size() + " icons to " + out);
    }

    static void text(Graphics2D g, String s, int cx, int baseline) {
        text(g, s, cx, baseline, 14f);
    }

    static void text(Graphics2D g, String s, int cx, int baseline, float size) {
        Font f = new Font(Font.SANS_SERIF, Font.BOLD, 10).deriveFont(size);
        g.setFont(f);
        int w = g.getFontMetrics().stringWidth(s);
        g.drawString(s, cx - w / 2f, baseline);
    }

    static void key(Graphics2D g, int x, int y, String label) {
        g.draw(new RoundRectangle2D.Double(x, y, 17, 17, 5, 5));
        text(g, label, x + 9, y + 13, 11f);
    }

    static void star(Graphics2D g, double cx, double cy, double r) {
        Path2D p = new Path2D.Double();
        for (int i = 0; i < 8; i++) {
            double a = Math.toRadians(i * 45 - 90);
            double d = i % 2 == 0 ? r : r * 0.32;
            if (i == 0) p.moveTo(cx + d * Math.cos(a), cy + d * Math.sin(a));
            else p.lineTo(cx + d * Math.cos(a), cy + d * Math.sin(a));
        }
        p.closePath();
        g.fill(p);
    }

    static void cloud(Graphics2D g, int dy) {
        Area a = new Area(new Ellipse2D.Double(6, 30 + dy, 24, 22));
        a.add(new Area(new Ellipse2D.Double(18, 16 + dy, 30, 30)));
        a.add(new Area(new Ellipse2D.Double(36, 28 + dy, 22, 24)));
        a.add(new Area(new Rectangle2D.Double(16, 38 + dy, 32, 14)));
        g.fill(a);
    }

    static void corner(Graphics2D g) {
        int a = 6, b = 58, l = 10;
        g.draw(new Line2D.Double(a, a, a + l, a)); g.draw(new Line2D.Double(a, a, a, a + l));
        g.draw(new Line2D.Double(b, a, b - l, a)); g.draw(new Line2D.Double(b, a, b, a + l));
        g.draw(new Line2D.Double(a, b, a + l, b)); g.draw(new Line2D.Double(a, b, a, b - l));
        g.draw(new Line2D.Double(b, b, b - l, b)); g.draw(new Line2D.Double(b, b, b, b - l));
    }
}
