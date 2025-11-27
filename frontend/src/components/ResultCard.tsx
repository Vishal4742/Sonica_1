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
                <h2 className="text-3xl font-bold leading-none mt-2">{match.title}</h2>
                <p className="text-xl text-gray-600 font-medium">{match.artist}</p>
            </div>
        </motion.div>
    );
};
