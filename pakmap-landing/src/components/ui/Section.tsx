"use client";

import { motion, useReducedMotion } from "framer-motion";

type SectionProps = {
  id?: string;
  children: React.ReactNode;
  className?: string;
  labelledBy?: string;
};

export function Section({ id, children, className = "", labelledBy }: SectionProps) {
  const reduce = useReducedMotion();

  return (
    <motion.section
      id={id}
      aria-labelledby={labelledBy}
      initial={reduce ? false : { opacity: 0, y: 16 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "-80px" }}
      transition={{ duration: 0.4, ease: [0.22, 1, 0.36, 1] }}
      className={className}
    >
      {children}
    </motion.section>
  );
}
