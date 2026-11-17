import { useCallback, useEffect, useRef, useState } from 'react';
import { AudioSourceToggle } from './components/AudioSourceToggle';
import { Crate, View } from './components/Crate';
import { MotionConfig } from 'framer-motion';
import { Mic, RotateCcw, Square } from 'lucide-react';
import { AudioSource, useAudioRecorder } from './hooks/useAudioRecorder';
import { useWebSocketAudio } from './hooks/useWebSocketAudio';
import { CleanMatch, cleanMatch } from './services/api';
import { formatTime } from './lib/time';

const LISTEN_SECS = 12; // the server gives up after this many 1 s slices

type Outcome =
    | { kind: 'match'; match: CleanMatch; took: number }
    | { kind: 'nomatch' }
    | { kind: 'blocked'; source: AudioSource };

function ListenClock({ start }: { start: number }) {
    const [now, setNow] = useState(Date.now());
    useEffect(() => {
        const id = setInterval(() => setNow(Date.now()), 250);
        return () => clearInterval(id);
    }, []);
    const sec = start ? Math.max(0, (now - start) / 1000) : 0;
    return (
        <div className="status" aria-hidden="true">
            <span>{formatTime(sec)}</span>
            <span className="bar"><i style={{ width: `${Math.min(100, (sec / LISTEN_SECS) * 100)}%` }} /></span>
        </div>
    );
}

function App() {
    const { isRecording, startContinuousRecording, stopRecording, stream } = useAudioRecorder();
    const { isConnected, connect, disconnect, startSession, sendChunk, lastMatch, noMatchAt } = useWebSocketAudio();

    const [audioSource, setAudioSource] = useState<AudioSource>('microphone');
    const [outcome, setOutcome] = useState<Outcome | null>(null);
    const [starting, setStarting] = useState(false); // waiting for the browser to grant the sound
    const [listenStart, setListenStart] = useState(0);
    const startRef = useRef(0);
    const active = useRef(false); // a listening session is running; replies after Stop are ignored

    const view: View = isRecording ? 'listening' : outcome ? outcome.kind : isConnected ? 'idle' : 'waking';

    // Keep one socket open; the hook reconnects (and wakes a sleeping server) on its own
    useEffect(() => {
        connect();
        return () => disconnect();
    }, [connect, disconnect]);

    const finish = useCallback((next: Outcome | null) => {
        active.current = false;
        stopRecording();
        setOutcome(next);
    }, [stopRecording]);

    useEffect(() => {
        if (lastMatch && active.current) {
            finish({ kind: 'match', match: cleanMatch(lastMatch), took: (Date.now() - startRef.current) / 1000 });
        }
    }, [lastMatch, finish]);

    useEffect(() => {
        if (noMatchAt && active.current) finish({ kind: 'nomatch' });
    }, [noMatchAt, finish]);

    // Audio sent while the socket is down is lost, so end the session and wait for it
    useEffect(() => {
        if (!isConnected && active.current) finish(null);
    }, [isConnected, finish]);

    const listen = useCallback(async () => {
        if (active.current) return finish(null); // Stop
        setOutcome(null);
        if (!isConnected) return connect();
        active.current = true;
        setStarting(true);
        startSession();
        const ok = await startContinuousRecording(audioSource, sendChunk, 1000);
        setStarting(false);
        if (!active.current) return stopRecording(); // stopped while the browser was asking
        if (!ok) {
            active.current = false;
            setOutcome({ kind: 'blocked', source: audioSource });
            return;
        }
        startRef.current = Date.now();
        setListenStart(startRef.current);
    }, [isConnected, connect, finish, startSession, startContinuousRecording, audioSource, sendChunk, stopRecording]);

    // Space starts and stops, unless a control has focus (it handles Space itself)
    useEffect(() => {
        const onKey = (e: KeyboardEvent) => {
            if (e.code !== 'Space' || e.repeat || view === 'waking' || starting) return;
            if ((e.target as HTMLElement).closest('button, a, input, textarea, select')) return;
            e.preventDefault();
            void listen();
        };
        window.addEventListener('keydown', onKey);
        return () => window.removeEventListener('keydown', onKey);
    }, [view, starting, listen]);

    const seg = <AudioSourceToggle audioSource={audioSource} onToggle={setAudioSource} />;
    const again = (label: string) => (
        <button className="btn" onClick={listen}><RotateCcw strokeWidth={1.75} />{label}</button>
    );

    let copy;
    if (view === 'waking') {
        copy = (<>
            <h1>Dig through your <em>crate.</em></h1>
            <p className="lede">The server is waking up. One moment.</p>
        </>);
    } else if (view === 'idle') {
        copy = (<>
            <h1>Dig through your <em>crate.</em></h1>
            <p className="lede">Play a song near your laptop, or share its tab. Sonica flips through every record in your library and pulls out the one that’s playing.</p>
            <div className="row">
                {seg}
                <button className="btn" onClick={listen} disabled={starting}><Mic strokeWidth={1.75} />Start digging</button>
            </div>
            <p className="hint">Scroll or drag the crate to flip by hand · Space to start</p>
        </>);
    } else if (view === 'listening') {
        copy = (<>
            <h2>Flipping <em>through…</em></h2>
            <p className="lede">The louder the song, the faster the records go. Keep it playing.</p>
            <div className="row"><button className="btn ghost" onClick={listen}><Square fill="currentColor" strokeWidth={0} />Stop</button></div>
        </>);
    } else if (outcome?.kind === 'match') {
        const m = outcome.match;
        copy = (<>
            <h2>Pulled it <em>out.</em></h2>
            <p className="lede">{m.untagged ? `${m.title}. Artist not tagged.` : `${m.title}, by ${m.artist}.`}</p>
            <div className="row">{again('Dig again')}</div>
        </>);
    } else if (outcome?.kind === 'nomatch') {
        copy = (<>
            <h2>Not in this <em>crate.</em></h2>
            <p className="lede">This record isn’t in your library. Try another part of the song, or another song.</p>
            <div className="row">{again('Dig again')}</div>
        </>);
    } else {
        const tab = outcome?.kind === 'blocked' && outcome.source === 'system';
        copy = (<>
            <h2>{tab ? <>No tab audio was <em>shared.</em></> : <>The microphone is <em>blocked.</em></>}</h2>
            <p className="lede">{tab
                ? 'When Chrome asks, pick the playing tab and turn on “Share tab audio”.'
                : 'Allow the microphone from the address bar, or switch to Tab audio.'}</p>
            <div className="row">{seg}{again('Try again')}</div>
        </>);
    }

    return (
        <MotionConfig reducedMotion="user">
            <a className="skip" href="#main">Skip to content</a>
            <Crate
                view={view}
                stream={stream}
                match={outcome?.kind === 'match' ? outcome.match : null}
                took={outcome?.kind === 'match' ? outcome.took : 0}
            />
            <header className="top">
                <a className="brand" href="/">sonica</a>
                <nav><a className="linkish" href="https://github.com/Vishal4742/Sonica_1">Source code</a></nav>
            </header>
            <main id="main" className="side" aria-live="polite">
                <section className="block" key={view}>{copy}</section>
            </main>
            {view === 'listening' && <ListenClock start={listenStart} />}
        </MotionConfig>
    );
}

export default App;
