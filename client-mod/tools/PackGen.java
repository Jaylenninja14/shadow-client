// Generates Shadow Client's own textures: the toggleable overlay packs (transparent overlays,
// clear glass/water) and the Shadow background (menu panorama for Java and Bedrock, plus a 16:9
// copy for the launcher's custom main menu). Nothing here is copied from Minecraft.
// Run from client-mod/: java tools/PackGen.java
import java.awt.image.BufferedImage;
import java.io.File;
import java.util.Iterator;
import java.util.Random;
import javax.imageio.IIOImage;
import javax.imageio.ImageIO;
import javax.imageio.ImageWriteParam;
import javax.imageio.ImageWriter;
import javax.imageio.stream.ImageOutputStream;

public class PackGen {
    static final String PACKS = "src/main/resources/resourcepacks";
    static final int FACE = 1024;

    public static void main(String[] args) throws Exception {
        String tex = "/assets/minecraft/textures/";
        write(transparent(16), PACKS + "/no_pumpkin" + tex + "misc/pumpkinblur.png");
        write(transparent(16), PACKS + "/no_underwater" + tex + "misc/underwater.png");
        write(transparent(16), PACKS + "/no_powder_snow" + tex + "misc/powder_snow_outline.png");
        write(glass(), PACKS + "/clear_glass" + tex + "block/glass.png");
        write(paneTop(), PACKS + "/clear_glass" + tex + "block/glass_pane_top.png");
        write(water(16, 1), PACKS + "/clear_water" + tex + "block/water_still.png");
        write(water(32, 2), PACKS + "/clear_water" + tex + "block/water_flow.png");
        write(hotbar(), PACKS + "/shadow_hotbar" + tex + "gui/sprites/hud/hotbar.png");
        write(hotbarSelection(), PACKS + "/shadow_hotbar" + tex + "gui/sprites/hud/hotbar_selection.png");

        for (int face = 0; face < 6; face++) {
            BufferedImage img = new BufferedImage(FACE, FACE, BufferedImage.TYPE_INT_RGB);
            for (int y = 0; y < FACE; y++) {
                for (int x = 0; x < FACE; x++) {
                    double u = (x + 0.5) / FACE * 2 - 1, v = 1 - (y + 0.5) / FACE * 2;
                    img.setRGB(x, y, paint(face, u, v));
                }
            }
            write(img, PACKS + "/shadow_background" + tex + "gui/title/background/panorama_" + face + ".png");
            write(img, "../crates/pc-core/data/bedrock/panorama_" + face + ".png");
        }
        // 16:9 view of the front face (horizon a little below the middle) for the launcher's menu
        int w = 1920, h = 1080;
        BufferedImage wide = new BufferedImage(w, h, BufferedImage.TYPE_INT_RGB);
        double top = 0.66, bottom = top - 2 * (double) h / w;
        for (int y = 0; y < h; y++) {
            for (int x = 0; x < w; x++) {
                double u = (x + 0.5) / w * 2 - 1, v = top + (bottom - top) * (y + 0.5) / h;
                wide.setRGB(x, y, paint(0, u, v));
            }
        }
        jpeg(wide, "../src/assets/shadow-background.jpg", 0.9f);
        System.out.println("textures written");
    }

    // ------------------------------------------------------------------ the Shadow background

    /**
     * One pixel of a cube face. Faces 0-3 are the sides, 4 the top, 5 the bottom. Everything that
     * varies along the horizon repeats per face and matches at both edges, so the faces join
     * whichever order and direction the game puts them in.
     */
    static int paint(int face, double u, double v) {
        double elev; // angle above the horizon
        if (face < 4) elev = Math.atan2(v, Math.sqrt(1 + u * u));
        else if (face == 4) elev = Math.asin(1 / Math.sqrt(u * u + v * v + 1));
        else elev = -Math.asin(1 / Math.sqrt(u * u + v * v + 1));

        double[] c = sky(elev);
        if (face < 4) {
            aurora(c, face, u, elev);
            stars(c, face, u, v, elev);
            if (face == 0) eclipse(c, u, v);
            shadowBlocks(c, face, u, v);
            terrain(c, face, u, elev);
        } else if (face == 4) {
            stars(c, face, u, v, elev);
        } else {
            mix(c, new double[] {7, 5, 13}, 1);
        }
        return rgb(c);
    }

    static double[] sky(double elev) {
        double[] horizon = {58, 29, 110}, mid = {18, 10, 36}, top = {5, 4, 11};
        double[] c;
        if (elev <= 0) c = horizon.clone();
        else if (elev < 0.35) c = lerp(horizon, mid, smooth(elev / 0.35));
        else c = lerp(mid, top, smooth(Math.min(1, (elev - 0.35) / 0.9)));
        double glow = Math.exp(-Math.abs(elev) * 9) * 0.35;
        add(c, new double[] {155, 123, 255}, glow);
        return c;
    }

    static void aurora(double[] c, int face, double u, double elev) {
        double phase = face * 1.7;
        double center = 0.30 + 0.05 * Math.cos(Math.PI * u) + 0.02 * Math.sin(2 * Math.PI * u + phase) * Math.sin(Math.PI * u);
        double band = Math.exp(-Math.pow((elev - center) / 0.05, 2));
        double ripple = 0.65 + 0.35 * Math.cos(6 * Math.PI * u + phase) * Math.cos(Math.PI * u / 2);
        add(c, new double[] {140, 90, 255}, band * 0.42 * ripple);
        add(c, new double[] {255, 110, 220}, Math.exp(-Math.pow((elev - center - 0.05) / 0.03, 2)) * 0.12 * ripple);
    }

    static void stars(double[] c, int face, double u, double v, double elev) {
        if (elev < 0.04) return;
        int k = 150;
        int cx = (int) Math.floor((u + 1) * k), cy = (int) Math.floor((v + 1) * k);
        long h = hash(face * 1_000_003L + cx * 7919L + cy * 104_729L);
        double density = 0.035 + 0.09 * Math.min(1, elev);
        if ((h & 0xFFFF) / 65535.0 > density) return;
        double sx = (cx + 0.2 + 0.6 * ((h >> 16 & 0xFF) / 255.0)) / k - 1, sy = (cy + 0.2 + 0.6 * ((h >> 24 & 0xFF) / 255.0)) / k - 1;
        double d = Math.hypot(u - sx, v - sy) * k;
        double size = 0.06 + 0.10 * ((h >> 32 & 0xFF) / 255.0);
        double bright = (h >> 40 & 0xFF) / 255.0;
        double b = Math.max(0, 1 - d / (size * 3)) * (0.25 + 0.75 * bright * bright);
        add(c, new double[] {235, 225, 255}, b * Math.min(1, elev * 6));
    }

    /** A dark disc with a violet corona and a bright rim, like the logo's eclipsed block. */
    static void eclipse(double[] c, double u, double v) {
        double cx = 0.42, cy = 0.30, r = 0.085;
        double d = Math.hypot(u - cx, v - cy);
        if (d < r) {
            mix(c, new double[] {6, 4, 12}, 1);
            double rim = Math.max(0, 1 - (r - d) / 0.012) * Math.max(0, ((cx - u) - (v - cy)) / r * 0.7);
            add(c, new double[] {217, 200, 255}, rim);
        } else {
            add(c, new double[] {155, 123, 255}, Math.exp(-(d - r) / 0.025) * 0.9);
            add(c, new double[] {90, 43, 217}, Math.exp(-(d - r) / 0.09) * 0.35);
        }
    }

    /** Floating dark blocks with two glowing eyes (the logo's shadow). */
    static void shadowBlocks(double[] c, int face, double u, double v) {
        double[][] blocks = face == 0 ? new double[][] {{-0.58, 0.16, 0.05}, {-0.30, 0.25, 0.022}} : face == 2 ? new double[][] {{0.35, 0.18, 0.04}} : new double[0][];
        for (double[] b : blocks) {
            double dx = u - b[0], dy = v - b[1], s = b[2];
            double outside = Math.max(Math.abs(dx), Math.abs(dy)) - s;
            if (outside < 0) {
                mix(c, new double[] {11, 9, 20}, 1);
                if (outside > -0.004) add(c, new double[] {91, 43, 217}, 0.7);
                for (int side = -1; side <= 1; side += 2) {
                    double ex = b[0] + side * s * 0.38, ey = b[1] + s * 0.12, es = s * 0.16;
                    if (Math.abs(u - ex) < es && Math.abs(v - ey) < es) mix(c, new double[] {155, 123, 255}, 1);
                }
            } else {
                add(c, new double[] {120, 80, 255}, Math.exp(-outside / 0.03) * 0.35);
            }
        }
    }

    /** Two layers of blocky hills; heights match at every face edge. */
    static void terrain(double[] c, int face, double u, double elev) {
        double far = height(face, u, 7, 0.09, 0.03, 48);
        double near = height(face, u, 3, 0.10, -0.02, 24);
        if (elev < far && elev >= near) {
            mix(c, new double[] {26, 18, 54}, 1);
            if (far - elev < 0.004) add(c, new double[] {120, 90, 230}, 0.35);
        }
        if (elev < near) {
            // dark ground made of faint blocks, a little lighter near the ridge
            double depth = Math.min(1, (near - elev) / 0.35);
            mix(c, lerp(new double[] {20, 14, 40}, new double[] {7, 5, 13}, smooth(depth)), 1);
            int bx = (int) Math.floor((u + 1) * 24), by = (int) Math.floor(elev / 0.04);
            double shade = ((hash(face * 977L + bx * 131L + by * 7L) & 0xFF) / 255.0 - 0.5) * 0.10 * (1 - depth);
            add(c, new double[] {120, 90, 230}, Math.max(0, shade));
            double fx = (u + 1) * 24 - bx, fy = elev / 0.04 - by;
            if ((fx < 0.04 || fy < 0.04) && depth < 0.8) add(c, new double[] {90, 60, 200}, 0.06 * (1 - depth));
            if (near - elev < 0.005) add(c, new double[] {155, 123, 255}, 0.45);
        }
    }

    static double height(int face, double u, int seed, double amp, double base, int columns) {
        Random r = new Random(seed * 31L + face);
        double[] a = new double[5], b = new double[3];
        for (int i = 0; i < a.length; i++) a[i] = (r.nextDouble() - 0.5) * amp / (i + 1);
        for (int i = 0; i < b.length; i++) b[i] = (r.nextDouble() - 0.5) * amp / (i + 1.5);
        double uq = (Math.floor((u + 1) / 2 * columns) + 0.5) / columns * 2 - 1; // blocky columns
        double edge = 0, h = 0;
        for (int i = 0; i < a.length; i++) {
            h += a[i] * Math.cos((i + 1) * Math.PI * uq);
            edge += a[i] * Math.cos((i + 1) * Math.PI);
        }
        for (int i = 0; i < b.length; i++) h += b[i] * Math.sin(2 * (i + 1) * Math.PI * uq);
        h += base - edge; // same height at both edges of every face
        return Math.round(h / 0.012) * 0.012;
    }

    // ------------------------------------------------------------------ overlay textures

    static BufferedImage transparent(int size) {
        return new BufferedImage(size, size, BufferedImage.TYPE_INT_ARGB);
    }

    /** 182x22 like the game's: dark, see-through slots with thin purple dividers. */
    static BufferedImage hotbar() {
        BufferedImage img = new BufferedImage(182, 22, BufferedImage.TYPE_INT_ARGB);
        for (int y = 0; y < 22; y++) {
            for (int x = 0; x < 182; x++) {
                boolean edge = x == 0 || y == 0 || x == 181 || y == 21;
                boolean divider = x > 0 && x < 181 && (x - 1) % 20 == 0 && y > 2 && y < 19;
                if (edge) img.setRGB(x, y, argb(200, 60, 40, 110));
                else if (divider) img.setRGB(x, y, argb(150, 110, 80, 200));
                else img.setRGB(x, y, argb(120, 8, 6, 16));
            }
        }
        return img;
    }

    /** 24x23 selection box: a bright purple frame. */
    static BufferedImage hotbarSelection() {
        BufferedImage img = new BufferedImage(24, 23, BufferedImage.TYPE_INT_ARGB);
        for (int y = 0; y < 23; y++) {
            for (int x = 0; x < 24; x++) {
                boolean outer = x == 0 || y == 0 || x == 23 || y == 22;
                boolean inner = x == 1 || y == 1 || x == 22 || y == 21;
                if (outer) img.setRGB(x, y, argb(255, 155, 123, 255));
                else if (inner) img.setRGB(x, y, argb(160, 217, 200, 255));
            }
        }
        return img;
    }

    static BufferedImage glass() {
        BufferedImage img = new BufferedImage(16, 16, BufferedImage.TYPE_INT_ARGB);
        for (int y = 0; y < 16; y++) {
            for (int x = 0; x < 16; x++) {
                boolean edge = x == 0 || y == 0 || x == 15 || y == 15;
                if (edge) img.setRGB(x, y, argb(200, 220, 236, 242));
            }
        }
        img.setRGB(2, 2, argb(140, 255, 255, 255));
        img.setRGB(3, 2, argb(110, 255, 255, 255));
        img.setRGB(2, 3, argb(110, 255, 255, 255));
        return img;
    }

    static BufferedImage paneTop() {
        BufferedImage img = new BufferedImage(16, 16, BufferedImage.TYPE_INT_ARGB);
        for (int y = 0; y < 16; y++) for (int x = 0; x < 16; x++) img.setRGB(x, y, argb(200, 220, 236, 242));
        return img;
    }

    /** Light grey (the game tints it by biome), more see-through than usual. */
    static BufferedImage water(int size, int seed) {
        Random r = new Random(seed);
        BufferedImage img = new BufferedImage(size, size, BufferedImage.TYPE_INT_ARGB);
        for (int y = 0; y < size; y++) {
            for (int x = 0; x < size; x++) {
                int g = 200 + r.nextInt(24);
                img.setRGB(x, y, argb(105, g, g, g));
            }
        }
        return img;
    }

    // ------------------------------------------------------------------ helpers

    static long hash(long x) {
        x ^= x >>> 33;
        x *= 0xff51afd7ed558ccdL;
        x ^= x >>> 33;
        x *= 0xc4ceb9fe1a85ec53L;
        x ^= x >>> 33;
        return x;
    }

    static double smooth(double t) {
        return t * t * (3 - 2 * t);
    }

    static double[] lerp(double[] a, double[] b, double t) {
        return new double[] {a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t};
    }

    static void add(double[] c, double[] col, double amount) {
        for (int i = 0; i < 3; i++) c[i] += col[i] * amount;
    }

    static void mix(double[] c, double[] col, double t) {
        for (int i = 0; i < 3; i++) c[i] = c[i] + (col[i] - c[i]) * t;
    }

    static int rgb(double[] c) {
        int r = (int) Math.max(0, Math.min(255, Math.round(c[0])));
        int g = (int) Math.max(0, Math.min(255, Math.round(c[1])));
        int b = (int) Math.max(0, Math.min(255, Math.round(c[2])));
        return 0xFF000000 | r << 16 | g << 8 | b;
    }

    static int argb(int a, int r, int g, int b) {
        return a << 24 | r << 16 | g << 8 | b;
    }

    static void write(BufferedImage img, String path) throws Exception {
        File f = new File(path);
        f.getParentFile().mkdirs();
        ImageIO.write(img, "png", f);
    }

    static void jpeg(BufferedImage img, String path, float quality) throws Exception {
        File f = new File(path);
        f.getParentFile().mkdirs();
        f.delete(); // the image stream doesn't truncate an existing file
        Iterator<ImageWriter> it = ImageIO.getImageWritersByFormatName("jpg");
        ImageWriter w = it.next();
        ImageWriteParam p = w.getDefaultWriteParam();
        p.setCompressionMode(ImageWriteParam.MODE_EXPLICIT);
        p.setCompressionQuality(quality);
        try (ImageOutputStream out = ImageIO.createImageOutputStream(f)) {
            w.setOutput(out);
            w.write(null, new IIOImage(img, null, null), p);
        }
        w.dispose();
    }
}
