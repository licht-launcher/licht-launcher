export const revealStyles = {
  root: "grid transition-[grid-template-rows,opacity] duration-200 ease-out motion-reduce:transition-none",
  rootExpanded: "grid-rows-[1fr] opacity-100",
  rootCollapsed: "grid-rows-[0fr] opacity-0",
  clip: "-mx-1 min-h-0 overflow-hidden px-1",
  content: "py-1",
} as const;
