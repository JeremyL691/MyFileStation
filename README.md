# MyFileStation 🚉

**The missing "File Shelf" for your Windows workflow.**

> *"Why do I have to minimize 10 windows just to drag a file to my Desktop?"* — Me, before building this.
> 

## 🤔 What is this?

You know that awkward moment when you're dragging a file from Chrome, and you realize the folder you want to drop it into is covered by 3 other windows? You hover there, shaking your mouse, hoping Windows gets the hint. It usually doesn't.

**MyFileStation** is a lightweight, edge-hiding "shelf" that lives on the side of your screen.

1. **Drag** a file to the screen edge ➡️ The station slides out.
2. **Drop** the file there (it stays safe).
3. **Open** your target app (Discord, Email, Photoshop).
4. **Drag** the file out from the station.

It's like a temporary "pocket" for your digital stuff.

## ✨ Features that actually matter

- **👻 Ghost Mode (Edge Sensing)**: It stays hidden and out of your way. It only pops out when you drag a file near the edge.
- **📋 Clipboard Magic**: Too lazy to drag? Just `Ctrl+C` a file, text, or screenshot, click the station, and `Ctrl+V`. It handles it all.
- **📌 Pin It**: Need to keep a file handy for later? Pin it so you don't accidentally swipe it away.
- **🖼️ Real Thumbnails**: No generic icons. See exactly what meme you are about to send.
- **⚡ Lightweight**: Built with Python, packaged as a standalone EXE. No bloatware.

## 🛠 Installation

**"I just want to use it" (For Users)**

1. Go to the [Releases](https://github.com/JeremyL691/MyFileStation/releases) page.
2. Download `MyFileStation.exe`.
3. Run it. That's it. No installer wizard, no "Next, Next, Finish".

**"I want to break it" (For Developers)**

```bash
# Clone the repo
git clone [<https://github.com/JeremyL691/MyFileStation.git>](<https://github.com/JeremyL691/MyFileStation.git>)

# Install dependencies
pip install -r requirements.txt

# Run source
python main.py
```

## 🎮 How to use

1. **Launch it.** You'll see a small bar on the screen edge (left/right).
2. **Drag a file** towards it -> It expands.
3. **Drop the file.**
4. **Right-click** on any item for more options (Open, Delete, Pin).
5. **Ctrl+V** inside the station to paste from your clipboard.

## 🏗 Tech Stack

- **Python 3.10+**: Because life is too short for C++.
- **Tkinter / PyQt** (depending on version): For the GUI.
- **PyInstaller**: To mash everything into a single `.exe`.

## ⚠️ Known Issues

- If it doesn't open, try running as Administrator (Windows permissions are fun).
- Works best on Windows 10/11. Mac/Linux support is... "theoretical" at this point.

## 📜 License

MIT License. Do whatever you want with it. Just don't blame me if your computer becomes self-aware.

---

*Built with ☕ and Python by JeremyL691.*

---