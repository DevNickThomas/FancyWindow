using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

// Win32 helpers for driving Fancy Window from PowerShell UI tests.
public static class UI
{
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }

    [StructLayout(LayoutKind.Sequential)]
    struct INPUT { public uint type; public InputUnion u; }
    [StructLayout(LayoutKind.Explicit)]
    struct InputUnion { [FieldOffset(0)] public MOUSEINPUT mi; [FieldOffset(0)] public KEYBDINPUT ki; }
    [StructLayout(LayoutKind.Sequential)]
    struct MOUSEINPUT { public int dx, dy; public uint mouseData, dwFlags, time; public IntPtr extra; }
    [StructLayout(LayoutKind.Sequential)]
    struct KEYBDINPUT { public ushort wVk, wScan; public uint dwFlags, time; public IntPtr extra; }

    delegate bool EnumProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc f, IntPtr l);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowTextW(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr h);
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] static extern bool GetClientRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] static extern bool ClientToScreen(IntPtr h, ref POINT p);
    [DllImport("user32.dll")] static extern IntPtr GetWindow(IntPtr h, uint cmd);
    [DllImport("user32.dll")] static extern IntPtr GetWindowLongPtrW(IntPtr h, int i);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] static extern bool MoveWindow(IntPtr h, int x, int y, int w, int hh, bool repaint);
    [DllImport("user32.dll")] static extern uint SendInput(uint n, INPUT[] i, int size);
    [DllImport("user32.dll")] static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] static extern int GetSystemMetrics(int i);
    [DllImport("user32.dll")] static extern IntPtr PostMessageW(IntPtr h, uint m, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] static extern bool IsIconic(IntPtr h);
    [DllImport("user32.dll")] static extern IntPtr GetWindowDC(IntPtr h);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassNameW(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] static extern IntPtr GetAncestor(IntPtr h, uint flags);

    [DllImport("user32.dll")] static extern IntPtr WindowFromPoint(POINT p);
    [DllImport("user32.dll")] static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int w, int hh, uint flags);

    static UI() { SetProcessDPIAware(); }

    /// The top-level window under a screen point.
    public static IntPtr RootAt(int x, int y) { return GetAncestor(WindowFromPoint(new POINT { X = x, Y = y }), 2 /*GA_ROOT*/); }

    /// Lifts a window (and its owned windows) to the top of the z-order without activating it.
    public static void Raise(IntPtr h)
    {
        const uint flags = 0x0001 | 0x0002 | 0x0010; // NOSIZE | NOMOVE | NOACTIVATE
        SetWindowPos(h, new IntPtr(-1), 0, 0, 0, 0, flags);
        SetWindowPos(h, new IntPtr(-2), 0, 0, 0, 0, flags);
    }

    public static string Title(IntPtr h) { var s = new StringBuilder(512); GetWindowTextW(h, s, 512); return s.ToString(); }
    public static string ClassName(IntPtr h) { var s = new StringBuilder(256); GetClassNameW(h, s, 256); return s.ToString(); }

    /// Visible top-level windows whose title equals `title`.
    public static IntPtr[] Find(string title)
    {
        var found = new List<IntPtr>();
        EnumWindows((h, l) => { if (IsWindowVisible(h) && Title(h) == title) found.Add(h); return true; }, IntPtr.Zero);
        return found.ToArray();
    }

    /// Visible top-level windows of a process (titles, for finding dialogs and popups).
    public static IntPtr[] OfProcess(uint pid)
    {
        var found = new List<IntPtr>();
        EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p == pid && IsWindowVisible(h)) found.Add(h); return true; }, IntPtr.Zero);
        return found.ToArray();
    }

    public static uint Pid(IntPtr h) { uint p; GetWindowThreadProcessId(h, out p); return p; }

    public static int[] Rect(IntPtr h) { RECT r; GetWindowRect(h, out r); return new[] { r.L, r.T, r.R - r.L, r.B - r.T }; }

    /// Client area in screen pixels: x, y, w, h.
    public static int[] Client(IntPtr h)
    {
        RECT r; GetClientRect(h, out r); var p = new POINT(); ClientToScreen(h, ref p);
        return new[] { p.X, p.Y, r.R, r.B };
    }

    public static IntPtr Owner(IntPtr h) { return GetWindow(h, 4); }
    public static long Style(IntPtr h) { return GetWindowLongPtrW(h, -16).ToInt64(); }
    public static long ExStyle(IntPtr h) { return GetWindowLongPtrW(h, -20).ToInt64(); }
    public static bool HasThickFrame(IntPtr h) { return (Style(h) & 0x00040000) != 0; }
    public static bool Minimized(IntPtr h) { return IsIconic(h); }
    public static void Place(IntPtr h, int x, int y, int w, int hh) { MoveWindow(h, x, y, w, hh, true); }
    public static void Close(IntPtr h) { PostMessageW(h, 0x0010, IntPtr.Zero, IntPtr.Zero); }

    /// Whether `a` is above `b` in the z-order.
    public static bool Above(IntPtr a, IntPtr b)
    {
        for (var h = GetWindow(b, 3 /*GW_HWNDPREV*/); h != IntPtr.Zero; h = GetWindow(h, 3)) if (h == a) return true;
        return false;
    }

    // ---- input -------------------------------------------------------------

    static void Send(params INPUT[] inputs) { SendInput((uint)inputs.Length, inputs, Marshal.SizeOf(typeof(INPUT))); }

    static INPUT MouseInput(uint flags, int dx = 0, int dy = 0)
    {
        return new INPUT { type = 0, u = new InputUnion { mi = new MOUSEINPUT { dx = dx, dy = dy, dwFlags = flags } } };
    }

    static INPUT KeyInput(ushort vk, bool up)
    {
        return new INPUT { type = 1, u = new InputUnion { ki = new KEYBDINPUT { wVk = vk, dwFlags = up ? 2u : 0u } } };
    }

    /// Absolute move over the whole virtual desktop.
    public static void MoveTo(int x, int y)
    {
        int vx = GetSystemMetrics(76), vy = GetSystemMetrics(77), vw = GetSystemMetrics(78), vh = GetSystemMetrics(79);
        int nx = (int)Math.Round((x - vx) * 65535.0 / (vw - 1));
        int ny = (int)Math.Round((y - vy) * 65535.0 / (vh - 1));
        Send(MouseInput(0x0001 | 0x8000 | 0x4000, nx, ny));
    }

    public static void LeftDown() { Send(MouseInput(0x0002)); }
    public static void LeftUp() { Send(MouseInput(0x0004)); }
    public static void RightDown() { Send(MouseInput(0x0008)); }
    public static void RightUp() { Send(MouseInput(0x0010)); }

    public static void KeyDown(ushort vk) { Send(KeyInput(vk, false)); }
    public static void KeyUp(ushort vk) { Send(KeyInput(vk, true)); }

    public static void Click(int x, int y, int pause = 80)
    {
        MoveTo(x, y); Thread.Sleep(pause); LeftDown(); Thread.Sleep(40); LeftUp(); Thread.Sleep(pause);
    }

    public static void RightClick(int x, int y, int pause = 80)
    {
        MoveTo(x, y); Thread.Sleep(pause); RightDown(); Thread.Sleep(40); RightUp(); Thread.Sleep(pause);
    }

    /// Presses modifiers, taps `key`, releases modifiers in reverse.
    public static void Chord(ushort[] modifiers, ushort key)
    {
        foreach (var m in modifiers) { KeyDown(m); Thread.Sleep(15); }
        KeyDown(key); Thread.Sleep(30); KeyUp(key); Thread.Sleep(15);
        for (int i = modifiers.Length - 1; i >= 0; i--) { KeyUp(modifiers[i]); Thread.Sleep(15); }
    }

    /// Left-drag from one point to another in steps, optionally holding modifier keys.
    public static void Drag(int x1, int y1, int x2, int y2, ushort[] hold, int steps = 25, int stepMs = 12)
    {
        hold = hold ?? new ushort[0]; // PowerShell turns an empty array argument into null
        MoveTo(x1, y1); Thread.Sleep(120);
        foreach (var m in hold) { KeyDown(m); Thread.Sleep(30); }
        LeftDown(); Thread.Sleep(150);
        for (int i = 1; i <= steps; i++)
        {
            MoveTo(x1 + (x2 - x1) * i / steps, y1 + (y2 - y1) * i / steps);
            Thread.Sleep(stepMs);
        }
        Thread.Sleep(150);
        LeftUp(); Thread.Sleep(150);
        // A bare Alt release would open the window menu; tap Ctrl first to cancel that.
        if (Array.IndexOf(hold, (ushort)0x12) >= 0) { KeyDown(0x11); KeyUp(0x11); }
        for (int i = hold.Length - 1; i >= 0; i--) { KeyUp(hold[i]); Thread.Sleep(20); }
    }

    // ---- pixels ------------------------------------------------------------

    public static void Shot(int x, int y, int w, int h, string path)
    {
        using (var bmp = new Bitmap(w, h, PixelFormat.Format24bppRgb))
        {
            using (var g = Graphics.FromImage(bmp)) g.CopyFromScreen(x, y, 0, 0, new Size(w, h));
            bmp.Save(path, ImageFormat.Png);
        }
    }

    /// Screen pixel colour as "#RRGGBB".
    public static string Pixel(int x, int y)
    {
        using (var bmp = new Bitmap(1, 1, PixelFormat.Format24bppRgb))
        {
            using (var g = Graphics.FromImage(bmp)) g.CopyFromScreen(x, y, 0, 0, new Size(1, 1));
            var c = bmp.GetPixel(0, 0);
            return string.Format("#{0:X2}{1:X2}{2:X2}", c.R, c.G, c.B);
        }
    }
}
