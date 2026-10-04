//! Sample processing in the stream callback.

/// TPDF dither of one LSB (`lsb`, 0 for formats that need none); silence stays silent.
pub(super) fn dither(sample: f32, lsb: f32, noise: &mut u32) -> f32 {
    if lsb == 0.0 || sample == 0.0 {
        return sample;
    }
    let mut uniform = || {
        // xorshift32
        *noise ^= *noise << 13;
        *noise ^= *noise >> 17;
        *noise ^= *noise << 5;
        (*noise >> 8) as f32 / (1 << 24) as f32
    };
    sample + (uniform() - uniform()) * lsb
}

/// Port of `opus_pcm_soft_clip` from libopus, Copyright (c) 2011 Xiph.Org Foundation, Skype Limited (BSD-3-Clause).
pub(super) fn soft_clip(samples: &mut [f32], channels: usize, mem: &mut [f32]) {
    if channels == 0 || samples.len() < channels {
        return;
    }
    let frames = samples.len() / channels;

    for sample in samples.iter_mut() {
        *sample = sample.clamp(-2.0, 2.0);
    }

    for (channel, declip) in mem.iter_mut().enumerate().take(channels) {
        let x = |i: usize| i * channels + channel;
        let mut a = *declip;

        for i in 0..frames {
            let v = samples[x(i)];
            if v * a >= 0.0 {
                break;
            }
            samples[x(i)] = v + a * v * v;
        }

        let mut curr = 0;
        let x0 = samples[x(0)];
        loop {
            let Some(i) = (curr..frames).find(|&i| samples[x(i)].abs() > 1.0) else {
                a = 0.0;
                break;
            };

            let pivot = samples[x(i)];
            let mut peak_pos = i;
            let mut start = i;
            let mut end = i;
            let mut maxval = pivot.abs();
            while start > 0 && pivot * samples[x(start - 1)] >= 0.0 {
                start -= 1;
            }
            while end < frames && pivot * samples[x(end)] >= 0.0 {
                if samples[x(end)].abs() > maxval {
                    maxval = samples[x(end)].abs();
                    peak_pos = end;
                }
                end += 1;
            }
            let special = start == 0 && pivot * samples[x(0)] >= 0.0;

            a = (maxval - 1.0) / (maxval * maxval);
            a += a * 2.4e-7;
            if pivot > 0.0 {
                a = -a;
            }
            for j in start..end {
                let v = samples[x(j)];
                samples[x(j)] = v + a * v * v;
            }

            if special && peak_pos >= 2 {
                let mut offset = x0 - samples[x(0)];
                let delta = offset / peak_pos as f32;
                for j in curr..peak_pos {
                    offset -= delta;
                    samples[x(j)] = (samples[x(j)] + offset).clamp(-1.0, 1.0);
                }
            }

            curr = end;
            if curr == frames {
                break;
            }
        }
        *declip = a;
    }
}
