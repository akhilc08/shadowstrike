use web_sys::{AudioContext, OscillatorType};

pub struct SoundEngine {
    ctx: Option<AudioContext>,
}

impl Default for SoundEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundEngine {
    pub fn new() -> Self {
        SoundEngine { ctx: None }
    }

    fn ensure_ctx(&mut self) -> Option<&AudioContext> {
        if self.ctx.is_none() {
            self.ctx = AudioContext::new().ok();
        }
        self.ctx.as_ref()
    }

    fn play_tone(&mut self, freq: f32, duration: f32, volume: f32, wave: OscillatorType) {
        let ctx = match self.ensure_ctx() {
            Some(c) => c,
            None => return,
        };
        let osc = match ctx.create_oscillator() {
            Ok(o) => o,
            Err(_) => return,
        };
        let gain = match ctx.create_gain() {
            Ok(g) => g,
            Err(_) => return,
        };
        osc.set_type(wave);
        osc.frequency().set_value(freq);
        gain.gain().set_value(volume);

        let now = ctx.current_time();
        // Quick attack, decay envelope
        gain.gain().set_value_at_time(0.0, now).ok();
        gain.gain().linear_ramp_to_value_at_time(volume, now + 0.01).ok();
        gain.gain().exponential_ramp_to_value_at_time(0.001, now + duration as f64).ok();

        let _ = osc.connect_with_audio_node(&gain);
        let _ = gain.connect_with_audio_node(&ctx.destination());
        osc.start().ok();
        osc.stop_with_when(now + duration as f64).ok();
    }

    fn play_noise_burst(&mut self, duration: f32, volume: f32) {
        // Use multiple detuned oscillators to approximate noise
        let ctx = match self.ensure_ctx() {
            Some(c) => c,
            None => return,
        };
        let gain = match ctx.create_gain() {
            Ok(g) => g,
            Err(_) => return,
        };
        let now = ctx.current_time();
        gain.gain().set_value_at_time(0.0, now).ok();
        gain.gain().linear_ramp_to_value_at_time(volume, now + 0.005).ok();
        gain.gain().exponential_ramp_to_value_at_time(0.001, now + duration as f64).ok();
        let _ = gain.connect_with_audio_node(&ctx.destination());

        for &freq in &[150.0, 280.0, 430.0, 590.0] {
            let osc = match ctx.create_oscillator() {
                Ok(o) => o,
                Err(_) => continue,
            };
            osc.set_type(OscillatorType::Square);
            osc.frequency().set_value(freq);
            let _ = osc.connect_with_audio_node(&gain);
            osc.start().ok();
            osc.stop_with_when(now + duration as f64).ok();
        }
    }

    /// Punch / light attack hit
    pub fn play_punch(&mut self) {
        self.play_tone(120.0, 0.08, 0.15, OscillatorType::Sawtooth);
        self.play_noise_burst(0.06, 0.08);
    }

    /// Sword clash / heavy hit
    pub fn play_sword_clash(&mut self) {
        self.play_tone(800.0, 0.06, 0.12, OscillatorType::Square);
        self.play_tone(1200.0, 0.04, 0.08, OscillatorType::Sawtooth);
        self.play_noise_burst(0.05, 0.1);
    }

    /// Jump whoosh
    pub fn play_jump(&mut self) {
        let ctx = match self.ensure_ctx() {
            Some(c) => c,
            None => return,
        };
        let osc = match ctx.create_oscillator() {
            Ok(o) => o,
            Err(_) => return,
        };
        let gain = match ctx.create_gain() {
            Ok(g) => g,
            Err(_) => return,
        };
        osc.set_type(OscillatorType::Sine);
        let now = ctx.current_time();
        osc.frequency().set_value_at_time(200.0, now).ok();
        osc.frequency().exponential_ramp_to_value_at_time(600.0, now + 0.1).ok();
        gain.gain().set_value_at_time(0.06, now).ok();
        gain.gain().exponential_ramp_to_value_at_time(0.001, now + 0.15).ok();
        let _ = osc.connect_with_audio_node(&gain);
        let _ = gain.connect_with_audio_node(&ctx.destination());
        osc.start().ok();
        osc.stop_with_when(now + 0.15).ok();
    }

    /// KO hit - big impact
    pub fn play_ko(&mut self) {
        self.play_tone(80.0, 0.3, 0.2, OscillatorType::Sawtooth);
        self.play_tone(60.0, 0.4, 0.15, OscillatorType::Sine);
        self.play_noise_burst(0.15, 0.15);
    }

    /// Round start bell
    pub fn play_bell(&mut self) {
        self.play_tone(800.0, 0.4, 0.1, OscillatorType::Sine);
        self.play_tone(1200.0, 0.3, 0.06, OscillatorType::Sine);
    }

    /// Block sound
    pub fn play_block(&mut self) {
        self.play_tone(300.0, 0.06, 0.1, OscillatorType::Square);
        self.play_noise_burst(0.04, 0.06);
    }

    /// Fireball cast
    pub fn play_fireball(&mut self) {
        let ctx = match self.ensure_ctx() {
            Some(c) => c,
            None => return,
        };
        let osc = match ctx.create_oscillator() {
            Ok(o) => o,
            Err(_) => return,
        };
        let gain = match ctx.create_gain() {
            Ok(g) => g,
            Err(_) => return,
        };
        osc.set_type(OscillatorType::Sawtooth);
        let now = ctx.current_time();
        osc.frequency().set_value_at_time(150.0, now).ok();
        osc.frequency().exponential_ramp_to_value_at_time(400.0, now + 0.15).ok();
        gain.gain().set_value_at_time(0.12, now).ok();
        gain.gain().exponential_ramp_to_value_at_time(0.001, now + 0.25).ok();
        let _ = osc.connect_with_audio_node(&gain);
        let _ = gain.connect_with_audio_node(&ctx.destination());
        osc.start().ok();
        osc.stop_with_when(now + 0.25).ok();
    }

    /// Dash strike whoosh
    pub fn play_dash(&mut self) {
        let ctx = match self.ensure_ctx() {
            Some(c) => c,
            None => return,
        };
        let osc = match ctx.create_oscillator() {
            Ok(o) => o,
            Err(_) => return,
        };
        let gain = match ctx.create_gain() {
            Ok(g) => g,
            Err(_) => return,
        };
        osc.set_type(OscillatorType::Sawtooth);
        let now = ctx.current_time();
        osc.frequency().set_value_at_time(400.0, now).ok();
        osc.frequency().exponential_ramp_to_value_at_time(100.0, now + 0.12).ok();
        gain.gain().set_value_at_time(0.1, now).ok();
        gain.gain().exponential_ramp_to_value_at_time(0.001, now + 0.12).ok();
        let _ = osc.connect_with_audio_node(&gain);
        let _ = gain.connect_with_audio_node(&ctx.destination());
        osc.start().ok();
        osc.stop_with_when(now + 0.12).ok();
        self.play_noise_burst(0.08, 0.08);
    }
}
