import os
from PIL import Image, ImageDraw

def create_ghost_icon():
    os.makedirs("src-tauri/icons", exist_ok=True)

    size = 512
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # Background glowing circular badge
    draw.ellipse([32, 32, 480, 480], fill=(15, 23, 42, 240), outline=(56, 189, 248, 220), width=16)

    # Cute ghost body
    # Head and body
    ghost_color = (241, 245, 249, 255)
    draw.pieslice([156, 120, 356, 320], start=180, end=0, fill=ghost_color)
    draw.rectangle([156, 220, 356, 360], fill=ghost_color)

    # Ghost ripples at bottom
    draw.ellipse([156, 340, 206, 380], fill=ghost_color)
    draw.ellipse([206, 340, 256, 380], fill=ghost_color)
    draw.ellipse([256, 340, 306, 380], fill=ghost_color)
    draw.ellipse([306, 340, 356, 380], fill=ghost_color)

    # Ghost eyes (glowing cyan)
    eye_color = (14, 165, 233, 255)
    draw.ellipse([200, 200, 225, 235], fill=eye_color)
    draw.ellipse([287, 200, 312, 235], fill=eye_color)

    # Musical note floating beside ghost
    note_color = (244, 114, 182, 255)
    # Stem
    draw.rectangle([340, 150, 350, 200], fill=note_color)
    draw.rectangle([380, 140, 390, 190], fill=note_color)
    draw.polygon([(340, 150), (390, 140), (390, 155), (340, 165)], fill=note_color)
    draw.ellipse([325, 190, 352, 210], fill=note_color)
    draw.ellipse([365, 180, 392, 200], fill=note_color)

    # Save 512x512 PNG
    img.save("src-tauri/icons/icon.png", format="PNG")
    img.resize((128, 128), Image.Resampling.LANCZOS).save("src-tauri/icons/128x128.png", format="PNG")
    img.resize((32, 32), Image.Resampling.LANCZOS).save("src-tauri/icons/32x32.png", format="PNG")

    # Save multi-size ICO
    ico_sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    img.save("src-tauri/icons/icon.ico", format="ICO", sizes=ico_sizes)

    print("Icons generated successfully in src-tauri/icons/")

if __name__ == "__main__":
    create_ghost_icon()
