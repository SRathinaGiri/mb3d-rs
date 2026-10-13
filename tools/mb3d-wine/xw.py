# X11 helper (ctypes): list windows, grab a window as PNG, click / type via XTest
import ctypes, ctypes.util, sys, subprocess, time
X = ctypes.CDLL("libX11.so.6"); T = ctypes.CDLL("libXtst.so.6")
X.XOpenDisplay.restype = ctypes.c_void_p
d = ctypes.c_void_p(X.XOpenDisplay(None))
X.XDefaultRootWindow.restype = ctypes.c_ulong; X.XDefaultRootWindow.argtypes = [ctypes.c_void_p]
root = X.XDefaultRootWindow(d)
class XImage(ctypes.Structure):
    _fields_ = [("width", ctypes.c_int), ("height", ctypes.c_int), ("xoffset", ctypes.c_int), ("format", ctypes.c_int), ("data", ctypes.POINTER(ctypes.c_ubyte)),
                ("byte_order", ctypes.c_int), ("bitmap_unit", ctypes.c_int), ("bitmap_bit_order", ctypes.c_int), ("bitmap_pad", ctypes.c_int), ("depth", ctypes.c_int),
                ("bytes_per_line", ctypes.c_int), ("bits_per_pixel", ctypes.c_int)]
X.XGetImage.restype = ctypes.POINTER(XImage)
X.XGetImage.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int, ctypes.c_int, ctypes.c_uint, ctypes.c_uint, ctypes.c_ulong, ctypes.c_int]
X.XQueryTree.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.POINTER(ctypes.c_ulong)), ctypes.POINTER(ctypes.c_uint)]
X.XFetchName.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.POINTER(ctypes.c_char_p)]
X.XGetGeometry.argtypes = [ctypes.c_void_p, ctypes.c_ulong] + [ctypes.POINTER(ctypes.c_ulong)] + [ctypes.POINTER(ctypes.c_int)]*2 + [ctypes.POINTER(ctypes.c_uint)]*4
X.XTranslateCoordinates.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_ulong, ctypes.c_int, ctypes.c_int, ctypes.POINTER(ctypes.c_int), ctypes.POINTER(ctypes.c_int), ctypes.POINTER(ctypes.c_ulong)]
def children(w):
    r = ctypes.c_ulong(); p = ctypes.c_ulong(); ch = ctypes.POINTER(ctypes.c_ulong)(); n = ctypes.c_uint()
    X.XQueryTree(d, w, ctypes.byref(r), ctypes.byref(p), ctypes.byref(ch), ctypes.byref(n))
    return [ch[i] for i in range(n.value)]
def name(w):
    s = ctypes.c_char_p(); X.XFetchName(d, w, ctypes.byref(s)); return s.value.decode("latin-1") if s.value else ""
def geom(w):
    r = ctypes.c_ulong(); x = ctypes.c_int(); y = ctypes.c_int(); wd = ctypes.c_uint(); h = ctypes.c_uint(); b = ctypes.c_uint(); dp = ctypes.c_uint()
    X.XGetGeometry(d, w, ctypes.byref(r), ctypes.byref(x), ctypes.byref(y), ctypes.byref(wd), ctypes.byref(h), ctypes.byref(b), ctypes.byref(dp))
    ax = ctypes.c_int(); ay = ctypes.c_int(); c = ctypes.c_ulong()
    X.XTranslateCoordinates(d, w, root, 0, 0, ctypes.byref(ax), ctypes.byref(ay), ctypes.byref(c))
    return ax.value, ay.value, wd.value, h.value
def find(title):
    for w in children(root):
        if name(w) == title: return w
        for c in children(w):
            if name(c) == title: return c
def grab(w, out):
    x, y, wd, h = geom(w)
    im = X.XGetImage(d, w, 0, 0, wd, h, 0xFFFFFFFF, 2).contents
    buf = ctypes.string_at(im.data, im.bytes_per_line * h)
    rgb = bytearray()
    for j in range(h):
        row = buf[j * im.bytes_per_line: j * im.bytes_per_line + wd * 4]
        for i in range(wd):
            rgb += bytes((row[4*i+2], row[4*i+1], row[4*i]))
    open(out + ".ppm", "wb").write(b"P6 %d %d 255\n" % (wd, h) + bytes(rgb))
    subprocess.run(["magick", out + ".ppm", out]); return (x, y, wd, h)
def click(x, y):
    T.XTestFakeMotionEvent(d, -1, x, y, 0); X.XFlush(d); time.sleep(0.15)
    T.XTestFakeButtonEvent(d, 1, 1, 0); X.XFlush(d); time.sleep(0.08)
    T.XTestFakeButtonEvent(d, 1, 0, 0); X.XFlush(d)
cmd = sys.argv[1]
if cmd == "list":
    for w in children(root):
        for c in [w] + children(w):
            n = name(c)
            if n: print(hex(c), repr(n), geom(c))
elif cmd == "grab":
    print(grab(find(sys.argv[2]), sys.argv[3]))
elif cmd == "click":
    click(int(sys.argv[2]), int(sys.argv[3]))
if cmd == "where":
    X.XQueryPointer.argtypes = [ctypes.c_void_p, ctypes.c_ulong] + [ctypes.POINTER(ctypes.c_ulong)]*2 + [ctypes.POINTER(ctypes.c_int)]*4 + [ctypes.POINTER(ctypes.c_uint)]
    r = ctypes.c_ulong(); c = ctypes.c_ulong(); rx = ctypes.c_int(); ry = ctypes.c_int(); wx = ctypes.c_int(); wy = ctypes.c_int(); m = ctypes.c_uint()
    if len(sys.argv) > 3:
        T.XTestFakeMotionEvent(d, -1, int(sys.argv[2]), int(sys.argv[3]), 0); X.XFlush(d); time.sleep(0.3)
    X.XQueryPointer(d, root, ctypes.byref(r), ctypes.byref(c), ctypes.byref(rx), ctypes.byref(ry), ctypes.byref(wx), ctypes.byref(wy), ctypes.byref(m))
    print("pointer", rx.value, ry.value, "child", hex(c.value), name(c.value) if c.value else "")
if cmd == "type":
    # types argv[2] (ASCII), then Return if argv[3] == "enter"
    X.XStringToKeysym.restype = ctypes.c_ulong; X.XStringToKeysym.argtypes = [ctypes.c_char_p]
    X.XKeysymToKeycode.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
    shift = X.XKeysymToKeycode(d, X.XStringToKeysym(b"Shift_L"))
    names = {'/': 'slash', '.': 'period', '_': 'underscore', '-': 'minus', ':': 'colon', '\\': 'backslash', ' ': 'space'}
    def key(code, sh=False):
        if sh: T.XTestFakeKeyEvent(d, shift, 1, 0)
        T.XTestFakeKeyEvent(d, code, 1, 0); T.XTestFakeKeyEvent(d, code, 0, 0)
        if sh: T.XTestFakeKeyEvent(d, shift, 0, 0)
        X.XFlush(d); time.sleep(0.03)
    for ch in sys.argv[2]:
        name = names.get(ch, ch)
        code = X.XKeysymToKeycode(d, X.XStringToKeysym(name.encode()))
        key(code, ch.isupper() or ch in '_:')
    if len(sys.argv) > 3 and sys.argv[3] == "enter":
        key(X.XKeysymToKeycode(d, X.XStringToKeysym(b"Return")))
