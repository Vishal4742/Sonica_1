import React from 'react';
import { motion } from 'framer-motion';

interface ListenButtonProps {
    isListening: boolean;
    onClick: () => void;
}

export const ListenButton: React.FC<ListenButtonProps> = ({ isListening, onClick }) => {
    return (
        <motion.button
            whileHover={{ scale: 1.02 }}
            whileTap={{ scale: 0.95, x: 5, y: 5, boxShadow: '5px 5px 0px #000' }}
            onClick={onClick}
            className={`
        w-[250px] h-[250px] 
        border-[3px] border-black 
        shadow-[10px_10px_0px_#000] 
        flex justify-center items-center 
        cursor-pointer transition-colors duration-200
        ${isListening ? 'bg-accent animate-pulse' : 'bg-primary'}
      `}
        >
            <span className="text-2xl font-bold uppercase tracking-wider">
                {isListening ? 'Listening...' : 'Identify'}
            </span>
        </motion.button>
    );
};
