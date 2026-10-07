"""Inspect generated WAVs and save a compact spectrogram review outside the repo.
Usage: python tools/inspect_examples.py <WINDFALL_STRETCH_RENDER_DIR>
Requires numpy, scipy and matplotlib, only for development verification.
"""
import csv
import sys
from pathlib import Path
import numpy as np
from scipy.io import wavfile
from scipy.signal import spectrogram
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

folder = Path(sys.argv[1])
rows = []
worst_cents = 0.0
worst_level = 0.0
settings = {"x050": (0.5, 0), "x075": (0.75, 0), "x125": (1.25, 0), "x150": (1.5, 0), "x200": (2, 0), "x400": (4, 0), "up12": (1, 12), "up4": (1, 4), "down5": (1, -5), "down12": (1, -12), "x130down3": (1.3, -3)}
for path in sorted(folder.glob("*.wav")):
    rate, data = wavfile.read(path)
    signal = data.astype(np.float64)
    assert rate == 48000 and np.isfinite(signal).all(), path
    name = path.stem.split("_")
    if len(name) == 3:
        ratio, semitones = settings[name[1]]
        _, source = wavfile.read(folder / (name[0] + ".wav"))
        assert len(signal) == round(len(source) * ratio), path
        if name[0] == "sine":
            part = signal[len(signal)//4:3*len(signal)//4]
            size = 1 << (len(part)*8 - 1).bit_length()
            power = np.abs(np.fft.rfft(part * np.hanning(len(part)), size))
            peak = np.argmax(power)
            a,b,c = np.log(power[peak-1:peak+2])
            delta = 0.5*(a-c)/(a-2*b+c)
            hz = (peak+delta)*rate/size
            wanted = 440*2**(semitones/12)
            cents = 1200*np.log2(hz/wanted)
            level = 20*np.log10(np.sqrt(np.mean(part*part))/(0.5/np.sqrt(2)))
            worst_cents = max(worst_cents, abs(cents))
            worst_level = max(worst_level, abs(level))
    rows.append((path.name, len(signal), np.max(np.abs(signal)), np.sqrt(np.mean(signal*signal)), np.mean(signal)))
with (folder / "inspection.csv").open("w", newline="") as f:
    writer = csv.writer(f)
    writer.writerow(["file", "frames", "peak", "rms", "mean"])
    writer.writerows(rows)
print(f"{len(rows)} WAVs: finite samples and exact transformed lengths. Sine worst {worst_cents:.5f} cents, {worst_level:.3f} dB (includes ratio 4).")
selected = ["sine", "sine_x150_standard", "sine_up12_high", "lowchord", "lowchord_x200_standard", "lowchord_x400_fast", "clicks", "clicks_x150_standard", "clicks_x400_fast", "noise", "noise_x150_standard", "song_sweep"]
fig, axes = plt.subplots(4,3, figsize=(15,12), constrained_layout=True)
for ax, name in zip(axes.flat, selected):
    rate, data = wavfile.read(folder / (name + ".wav"))
    freq, time, power = spectrogram(data, rate, nperseg=1024, noverlap=768)
    ax.pcolormesh(time, freq, 10*np.log10(power+1e-14), vmin=-90, vmax=-25, shading="auto", cmap="magma")
    ax.set_ylim(0, 2500 if "chord" in name else 12000)
    ax.set_title(name)
    ax.set_xlabel("Time (s)")
    ax.set_ylabel("Frequency (Hz)")
fig.savefig(folder / "spectrogram-review.png", dpi=120)
