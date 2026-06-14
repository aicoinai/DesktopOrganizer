from PIL import Image, ImageDraw, ImageFont
import struct, os, io

def draw_d(size):
    img = Image.new('RGBA', (size, size), (0, 0, 0, 255))
    draw = ImageDraw.Draw(img)
    try:
        font_size = max(10, int(size * 0.55))
        font = ImageFont.truetype('C:/Windows/Fonts/seguisb.ttf', font_size)
    except Exception:
        font = ImageFont.load_default()
    bbox = draw.textbbox((0, 0), 'D', font=font)
    tw = bbox[2] - bbox[0]
    th = bbox[3] - bbox[1]
    x = (size - tw) // 2 - bbox[0]
    y = (size - th) // 2 - bbox[1]
    draw.text((x, y), 'D', fill=(255, 255, 255, 255), font=font)
    return img

def create_png_ico(sizes, output_path):
    """Create ICO with PNG-compressed images (modern Windows RC accepts these)"""
    os.makedirs(os.path.dirname(output_path) or '.', exist_ok=True)

    # First create PNG data for each size
    images = []
    for s in sizes:
        img = draw_d(s)
        buf = io.BytesIO()
        img.save(buf, format='PNG', compress_level=0)  # no compression overhead
        images.append((s, buf.getvalue()))

    num = len(images)
    # ICONDIR = 6 bytes, each entry = 16 bytes
    offset = 6 + num * 16

    with open(output_path, 'wb') as f:
        # ICONDIR header
        f.write(struct.pack('<HHB', 0, 1, num))
        for s, png_data in images:
            f.write(struct.pack('<BBBBHHII',
                s if s < 256 else 0,  # width (0 = 256)
                s if s < 256 else 0,  # height (0 = 256)
                0,    # colors (0 = no palette)
                0,    # reserved
                1,    # color planes
                32,   # bits per pixel
                len(png_data),  # size of PNG data
                offset))         # offset to PNG data
            offset += len(png_data)
        for _, png_data in images:
            f.write(png_data)

    print(f'Created {output_path} with {num} PNG-compressed images')
    # Verify
    with open(output_path, 'rb') as f:
        data = f.read(4)
        print(f'  Magic: {data.hex()} (expect 00000100 for ICO)')

create_png_ico([16, 32, 48], 'assets/app_icon.ico')

# Also create a 256x256 version and save as separate high-res icon
img256 = draw_d(256)
img256.save('assets/app_icon_256.png', format='PNG')
print('Created 256px PNG reference')
