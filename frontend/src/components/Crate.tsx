import { useEffect, useRef } from 'react';
import { motion } from 'framer-motion';
import { CleanMatch } from '../services/api';
import { formatTime } from '../lib/time';

export type View = 'waking' | 'idle' | 'listening' | 'match' | 'nomatch' | 'blocked';

const COVERS = ['sunset', 'folk', 'tabla', 'city', 'marigold', 'harmonium'];
const N = 18; // sleeves in the crate; the covers repeat
const EASE = [0.22, 1, 0.36, 1] as const;

// Bass loudness 0–1 from the lowest 16 of 96 log-spaced bands between 60 Hz and 8 kHz,
// the same scale the design demo was tuned on
const FMIN = 60, FMAX = 8000, BINS = 96;
function bassLevel(analyser: AnalyserNode, buf: Float32Array, sampleRate: number) {
    analyser.getFloatFrequencyData(buf);
    const nyq = sampleRate / 2, n = buf.length, dy = 0.5 / BINS;
    let sum = 0;
    for (let b = 0; b < 16; b++) {
        const y = b / (BINS - 1);
        const b0 = Math.max(1, Math.floor((FMIN * Math.pow(FMAX / FMIN, y - dy)) / nyq * n));
        const b1 = Math.min(n - 1, Math.ceil((FMIN * Math.pow(FMAX / FMIN, y + dy)) / nyq * n));
        let m = -140;
        for (let i = b0; i <= b1; i++) if (buf[i] > m) m = buf[i];
        const v = Math.min(1, Math.max(0, (m + 92) / 62));
        sum += v * v;
    }
    return sum / 16;
}

interface CrateProps {
    view: View;
    stream: MediaStream | null;
    match: CleanMatch | null;
    took: number; // seconds of listening before the match
}

export const Crate = ({ view, stream, match, took }: CrateProps) => {
    const shopRef = useRef<HTMLDivElement>(null);
    const sleeves = useRef<(HTMLDivElement | null)[]>([]);
    const viewRef = useRef(view);
    const audio = useRef<{ analyser: AnalyserNode; buf: Float32Array; rate: number } | null>(null);
    const vel = useRef(0);
    viewRef.current = view;

    // Listen to the same stream that is being recorded; nothing is played back
    useEffect(() => {
        if (!stream) return;
        const ctx = new AudioContext();
        const analyser = ctx.createAnalyser();
        analyser.fftSize = 4096;
        analyser.smoothingTimeConstant = 0.15;
        ctx.createMediaStreamSource(stream).connect(analyser);
        void ctx.resume();
        audio.current = { analyser, buf: new Float32Array(analyser.frequencyBinCount), rate: ctx.sampleRate };
        return () => {
            audio.current = null;
            void ctx.close();
        };
    }, [stream]);

    // Flip by hand: wheel or drag
    useEffect(() => {
        const shop = shopRef.current!;
        let dragY: number | null = null;
        const wheel = (e: WheelEvent) => { vel.current += e.deltaY * 0.0009; };
        const down = (e: PointerEvent) => { dragY = e.clientY; shop.setPointerCapture(e.pointerId); };
        const move = (e: PointerEvent) => { if (dragY === null) return; vel.current += (e.clientY - dragY) * 0.004; dragY = e.clientY; };
        const up = () => { dragY = null; };
        shop.addEventListener('wheel', wheel, { passive: true });
        shop.addEventListener('pointerdown', down);
        shop.addEventListener('pointermove', move);
        shop.addEventListener('pointerup', up);
        shop.addEventListener('pointercancel', up);
        return () => {
            shop.removeEventListener('wheel', wheel);
            shop.removeEventListener('pointerdown', down);
            shop.removeEventListener('pointermove', move);
            shop.removeEventListener('pointerup', up);
            shop.removeEventListener('pointercancel', up);
        };
    }, []);

    // One animation loop for every sleeve
    useEffect(() => {
        const reduced = window.matchMedia('(prefers-reduced-motion: reduce)');
        let f = 0, raf = 0;
        const frame = () => {
            const st = viewRef.current, a = audio.current;
            const bass = a ? bassLevel(a.analyser, a.buf, a.rate) : 0;
            // Louder songs flip faster while listening; a slow browse at rest
            const target = reduced.matches ? 0 : st === 'listening' ? 0.02 + bass * 0.12 : st === 'idle' ? 0.004 : 0;
            vel.current += (target - vel.current) * 0.06;
            f = (f + vel.current + N) % N;
            const dimmed = st === 'match' || st === 'nomatch';
            for (let k = 0; k < N; k++) {
                const el = sleeves.current[k];
                if (!el) continue;
                let rel = k - f;
                if (rel < -1.5) rel += N;
                if (rel < 0) {
                    const p = Math.min(1, -rel);
                    el.style.transform = `translateZ(${-rel * 4}px) rotateX(${-8 + p * 92}deg)`;
                    el.style.opacity = String(1 - p * 0.9);
                    el.style.zIndex = '50';
                } else if (rel > 9) {
                    el.style.opacity = '0';
                } else {
                    // Records further back sit higher and darker so their top edges read as a stack
                    el.style.transform = `translateZ(${-rel * 22}px) translateY(${-rel * 16}px) rotateX(-8deg)`;
                    el.style.opacity = String(1 - rel * 0.04);
                    el.style.zIndex = String(40 - Math.round(rel));
                }
                const dim = Math.max(0.45, 1 - Math.max(0, rel) * 0.07);
                el.style.filter = `brightness(${dimmed ? dim * 0.55 : dim})`;
            }
            raf = requestAnimationFrame(frame);
        };
        raf = requestAnimationFrame(frame);
        return () => cancelAnimationFrame(raf);
    }, []);

    return (
        <>
            <div className="shop" ref={shopRef} aria-hidden="true">
                <div className="crate">
                    {Array.from({ length: N }, (_, k) => N - 1 - k).map((k) => (
                        <div
                            key={k}
                            className="sleeve"
                            ref={(el) => { sleeves.current[k] = el; }}
                            style={{ backgroundImage: `url(/covers/${COVERS[(k * 5) % 6]}.webp)` }}
                        />
                    ))}
                    <div className="box" />
                </div>
            </div>
            {/* A match lifts the record out of the crate, then slides the vinyl out of its sleeve */}
            <motion.div
                className="pick"
                aria-hidden="true"
                initial={false}
                style={{ x: '-50%', rotate: -4 }}
                animate={view === 'match' ? { opacity: 1, y: '0vh', scale: 1 } : { opacity: 0, y: '40vh', scale: 0.9 }}
                transition={{ duration: 0.9, ease: EASE, opacity: { duration: 0.4 } }}
            >
                <motion.div
                    className="disc"
                    initial={false}
                    animate={{ x: view === 'match' ? '46%' : '0%' }}
                    transition={{ duration: 1, ease: EASE, delay: view === 'match' ? 0.5 : 0 }}
                />
                <div className="back">
                    {match && (
                        <>
                            <small>Found in your library</small>
                            <div>
                                <div className={`t ${match.title.length > 44 ? 'l' : ''}`}>{match.title}</div>
                                <div className="a">{match.artist}</div>
                            </div>
                            <div className="m"><span>{match.pct}% sure</span><span>heard {formatTime(took)}</span></div>
                        </>
                    )}
                </div>
            </motion.div>
        </>
    );
};
