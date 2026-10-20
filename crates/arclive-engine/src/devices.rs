use arclive_core::{DeviceKind, DeviceSpec};

/// Process one interleaved stereo buffer in place. RT-safe: no allocation.
pub fn process_device_chain(devices: &[DeviceSpec], interleaved: &mut [f32], channels: usize) {
    for device in devices {
        if device.bypass {
            continue;
        }
        match device.kind {
            DeviceKind::Gain => apply_gain(interleaved, device.params[0]),
            DeviceKind::Filter => apply_one_pole_lpf(interleaved, channels, device.params[0]),
            DeviceKind::Eq => apply_simple_eq(interleaved, channels, device.params),
        }
    }
}

fn apply_gain(buf: &mut [f32], amount: f32) {
    // amount 0..1 → -24..+12 dB-ish linear scale
    let g = (amount * 2.0).clamp(0.0, 2.0);
    for s in buf.iter_mut() {
        *s *= g;
    }
}

#[allow(clippy::manual_memcpy)]
fn apply_one_pole_lpf(buf: &mut [f32], channels: usize, cutoff: f32) {
    let alpha = cutoff.clamp(0.01, 0.99);
    if channels == 0 {
        return;
    }
    let mut state = [0.0_f32; 8];
    let ch = channels.min(8);
    for frame in buf.chunks_exact_mut(channels) {
        for c in 0..ch {
            let x = frame[c];
            state[c] += alpha * (x - state[c]);
            frame[c] = state[c];
        }
    }
}

fn apply_simple_eq(buf: &mut [f32], channels: usize, params: [f32; 4]) {
    // Three bands as simple shelving gains (low/mid/high) — educational EQ.
    let low_g = params[0] * 2.0;
    let mid_g = params[1] * 2.0;
    let high_g = params[2] * 2.0;
    let mut low = [0.0_f32; 8];
    let mut high = [0.0_f32; 8];
    let ch = channels.min(8);
    let a_low = 0.05_f32;
    let a_high = 0.4_f32;
    for frame in buf.chunks_exact_mut(channels) {
        for c in 0..ch {
            let x = frame[c];
            low[c] += a_low * (x - low[c]);
            high[c] += a_high * (x - high[c]);
            let high_band = x - high[c];
            let mid = x - low[c] - high_band;
            frame[c] = low[c] * low_g + mid * mid_g + high_band * high_g;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arclive_core::DeviceSpec;

    #[test]
    fn gain_changes_amplitude() {
        let mut buf = [0.5_f32, 0.5];
        let dev = DeviceSpec::gain();
        process_device_chain(&[dev], &mut buf, 2);
        assert!(buf[0] > 0.5);
    }
}
