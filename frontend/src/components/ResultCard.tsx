import React from 'react';
import { motion } from 'framer-motion';
import { SongMatch } from '../services/api';
import { X } from 'lucide-react';

interface ResultCardProps {
    match: SongMatch | null;
    onClose: () => void;
}

export const ResultCard: React.FC<ResultCardProps> = ({ match, onClose }) => {
    if (!match) return null;

    // Helper to clean up messy filenames/metadata
    const cleanMetadata = (title: string, artist: string) => {
        let cleanTitle = title;
        let cleanArtist = artist;

        // 1. Remove common junk tags (case insensitive)
        const junkRegex = /\(MP3_.*?\)|\[.*?\]|\(.*?kbps.*?\)|www\..*?\.[a-z]+|\.mp3|\.wav/gi;
        cleanTitle = cleanTitle.replace(junkRegex, '').trim();

        // 2. Replace underscores and multiple spaces
        cleanTitle = cleanTitle.replace(/_/g, ' ').replace(/\s+/g, ' ').trim();

        // 3. If artist is "Unknown", try to extract from title
        if (cleanArtist === 'Unknown' || cleanArtist === '') {
            // Check for " - " separator
            if (cleanTitle.includes(' - ')) {
                const parts = cleanTitle.split(' - ');
                if (parts.length >= 2) {
                    cleanArtist = parts[0].trim();
                    cleanTitle = parts.slice(1).join(' - ').trim();
                }
            }
            // Check for " _ " separator (sometimes used as -)
            else if (cleanTitle.includes(' _ ')) {
                const parts = cleanTitle.split(' _ ');
                if (parts.length >= 2) {
                    cleanArtist = parts[0].trim();
                    cleanTitle = parts.slice(1).join(' _ ').trim();
                }
            }
        }

        return { title: cleanTitle, artist: cleanArtist };
    };

    const { title, artist } = cleanMetadata(match.title, match.artist);

    return (
        <motion.div
            initial={{ y: 100, opacity: 0 }}
            animate={{ y: 0, opacity: 1 }}
            exit={{ y: 100, opacity: 0 }}
            className="absolute bottom-[100px] left-0 right-0 mx-auto w-[90%] max-w-[400px] bg-white border-[3px] border-black shadow-[8px_8px_0px_#000] p-6 z-50"
        >
            <button
                onClick={onClose}
                className="absolute top-2 right-2 p-1 hover:bg-gray-200 border border-transparent hover:border-black transition-all"
            >
                <X size={20} />
            </button>

            <div className="flex flex-col gap-2">
                <div className="text-xs font-bold uppercase bg-accent inline-block self-start px-2 py-1 border border-black">
                    Match Found ({Math.min(100, Math.max(0, Math.round(match.score * 100)))}%)
                </div>
                <h2 className="text-3xl font-bold leading-none mt-2 break-words line-clamp-2" title={title}>{title}</h2>
                <p className="text-xl text-gray-600 font-medium line-clamp-1" title={artist}>{artist}</p>
            </div>
        </motion.div>
    );
};
