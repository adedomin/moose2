#!/usr/bin/env python3
from PIL import Image
from sys import argv, exit

def usage():
    print("usage: make-favicon.py 16x16.png [32x32.png ...] output.ico"
          "\n"
          "Make sure to optimize your PNGs first, using something like oxipng, optipng, pngcrush and others.")
    exit(1)

if len(argv[1:]) < 2:
    usage()

output = argv[-1]
if not output.endswith('.ico'):
    usage()

images = []
for arg in argv[1:-1]: 
    images.append(Image.open(arg))

last = images[-1]
last.save(output,
          sizes=list(map(lambda img: img.size, images)),
          append_image=images[:-1])
