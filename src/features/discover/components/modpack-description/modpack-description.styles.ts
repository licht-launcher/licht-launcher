/** Typography for author-written HTML, scoped to the description container. */
export const modpackDescriptionStyles = {
  root: [
    "min-w-0 text-sm leading-relaxed break-words text-foreground",
    "[&_h1]:mt-5 [&_h1]:mb-2 [&_h1]:text-xl [&_h1]:font-semibold",
    "[&_h2]:mt-5 [&_h2]:mb-2 [&_h2]:text-lg [&_h2]:font-semibold",
    "[&_h3]:mt-4 [&_h3]:mb-2 [&_h3]:text-base [&_h3]:font-semibold",
    "[&_h4]:mt-3 [&_h4]:mb-1 [&_h4]:font-medium [&_h5]:font-medium [&_h6]:font-medium",
    "[&_p]:my-2 [&_ul]:my-2 [&_ul]:list-disc [&_ul]:pl-5 [&_ol]:my-2 [&_ol]:list-decimal [&_ol]:pl-5",
    "[&_a]:text-primary [&_a]:underline [&_a]:underline-offset-4 [&_a]:hover:text-primary/80",
    "[&_a]:focus-visible:rounded-sm [&_a]:focus-visible:ring-3 [&_a]:focus-visible:ring-ring/50 [&_a]:focus-visible:outline-none",
    "[&_img]:inline-block [&_img]:h-auto [&_img]:max-w-full [&_img]:rounded-md",
    "[&_blockquote]:my-2 [&_blockquote]:border-l-2 [&_blockquote]:border-border [&_blockquote]:pl-3 [&_blockquote]:text-muted-foreground",
    "[&_code]:rounded [&_code]:bg-muted [&_code]:px-1 [&_code]:text-xs [&_pre]:my-2 [&_pre]:overflow-x-auto [&_pre]:rounded-md [&_pre]:bg-muted [&_pre]:p-3",
    "[&_hr]:my-4 [&_hr]:border-border",
    "[&_table]:my-2 [&_table]:w-full [&_td]:border [&_td]:border-border [&_td]:px-2 [&_td]:py-1 [&_th]:border [&_th]:border-border [&_th]:px-2 [&_th]:py-1 [&_th]:text-left",
  ].join(" "),
} as const;
