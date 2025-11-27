import { Mic, Monitor } from 'lucide-react';

interface AudioSourceToggleProps {
    audioSource: 'microphone' | 'system';
    onToggle: (source: 'microphone' | 'system') => void;
    disabled?: boolean;
}

export const AudioSourceToggle = ({ audioSource, onToggle, disabled }: AudioSourceToggleProps) => {
    return (
        <div className="flex items-center gap-2 bg-white border-2 border-black p-1">
            <button
                onClick={() => onToggle('microphone')}
                disabled={disabled}
                className={`
                    flex items-center gap-2 px-4 py-2 font-bold uppercase text-sm transition-all
                    ${audioSource === 'microphone'
                        ? 'bg-accent text-black border-2 border-black'
                        : 'bg-white text-gray-600 hover:bg-gray-100'
                    }
                    ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}
                `}
            >
                <Mic size={18} />
                Microphone
            </button>
            <button
                onClick={() => onToggle('system')}
                disabled={disabled}
                className={`
                    flex items-center gap-2 px-4 py-2 font-bold uppercase text-sm transition-all
                    ${audioSource === 'system'
                        ? 'bg-accent text-black border-2 border-black'
                        : 'bg-white text-gray-600 hover:bg-gray-100'
                    }
                    ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}
                `}
            >
                <Monitor size={18} />
                System Audio
            </button>
        </div>
    );
};
