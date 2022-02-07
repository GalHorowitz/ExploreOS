from PIL import Image

CHAR_WIDTH = 12

data = []
with Image.open("jetbrains_mono_ascii_downsampled_cropped.png") as im:
	im = im.convert('L')
	width, height = im.size
	num_chars = width // CHAR_WIDTH
	pixels = im.load()
	for i in range(num_chars):
		base_x = CHAR_WIDTH*i
		for y in range(height):
			for x in range(CHAR_WIDTH):
				data.append(pixels[base_x+x, y])

with open('compact_font.bin', 'wb') as f:
	f.write(bytes(data))
