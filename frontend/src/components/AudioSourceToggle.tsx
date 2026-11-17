import { AudioSource } from '../hooks/useAudioRecorder';
import { Mic, Monitor } from 'lucide-react';

interface AudioSourceToggleProps {
    audioSource: AudioSource;
    onToggle: (source: AudioSource) => void;
}

export const AudioSourceToggle = ({ audioSource, onToggle }: AudioSourceToggleProps) => (
    <div className="seg" role="radiogroup" aria-label="Where the sound comes from">
        <button role="radio" aria-checked={audioSource === 'microphone'} onClick={() => onToggle('microphone')}>
            <Mic strokeWidth={1.75} />Microphone
        </button>
        <button role="radio" aria-checked={audioSource === 'system'} onClick={() => onToggle('system')}>
            <Monitor strokeWidth={1.75} />Tab audio
        </button>
    </div>
);
