import struct, os

def draw_grid_icon(size):
    """Draw a 3x3 grid icon: 3x3 squares with center empty, black bg, white squares"""
    buf = bytearray(size * size * 4)
    margin = max(1, size // 8)
    cell = (size - margin * 2) // 3
    gap  = max(1, cell // 5)
    sq   = cell - gap

    for y in range(size):
        for x in range(size):
            idx = (y * size + x) * 4
            cy_id = (y - margin) // cell
            cx_id = (x - margin) // cell
            in_cy = margin + cy_id * cell
            in_cx = margin + cx_id * cell
            in_sq = (in_cy <= y < in_cy + sq) and (in_cx <= x < in_cx + sq)
            center = (cy_id == 1 and cx_id == 1)

            if in_sq and not center:
                buf[idx:idx+4] = [255, 255, 255, 255]  # BGRA white
            else:
                buf[idx:idx+4] = [0, 0, 0, 255]        # BGRA black
    return bytes(buf)

def pack_bmp_rows(size, bgra_data):
    """Pack BGRA rows bottom-up with 4-byte row alignment"""
    rows = []
    for y in range(size-1, -1, -1):
        row = bgra_data[y*size*4:(y+1)*size*4]
        pad = (4 - (size * 4) % 4) % 4
        rows.append(row + b'\x00' * pad)
    return b''.join(rows)

def create_ico(sizes, output_path):
    images = []
    for s in sizes:
        bmp_data = pack_bmp_rows(s, draw_grid_icon(s))
        # BITMAPINFOHEADER (40 bytes)
        bih = struct.pack('<IIIHHIIIIII',
            40,           # biSize
            s,            # biWidth
            s * 2,        # biHeight (XOR + AND mask)
            1,            # biPlanes
            32,           # biBitCount (BGRA)
            0,            # biCompression (BI_RGB)
            len(bmp_data), # biSizeImage
            0, 0, 0, 0)  # resolution + colors
        images.append((s, bih + bmp_data))

    num = len(images)
    dir_entry_size = 16 * num
    offset = 6 + dir_entry_size

    with open(output_path, 'wb') as f:
        # ICONDIR header
        f.write(struct.pack('<HHB', 0, 1, num))
        for s, data in images:
            f.write(struct.pack('<BBBBHHII',
                s if s < 256 else 0,
                s if s < 256 else 0,
                0, 0, 1, 32,
                len(data), offset))
            offset += len(data)
        for _, data in images:
            f.write(data)

os.makedirs('assets', exist_ok=True)
create_ico([16, 32, 48], 'assets/app_icon.ico')
print('Created assets/app_icon.ico')
