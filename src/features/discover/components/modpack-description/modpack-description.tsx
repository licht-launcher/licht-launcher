import parse, {
  type DOMNode,
  domToReact,
  Element,
  type HTMLReactParserOptions,
} from "html-react-parser";
import { useMemo } from "react";

import { openLink } from "@/features/discover/lib/open-link";

import { modpackDescriptionStyles } from "./modpack-description.styles";

/**
 * Renders a description the backend already sanitized. Links open in the
 * system browser, so a click never navigates the launcher's webview away.
 */
export function ModpackDescription({ html }: { html: string }) {
  const content = useMemo(() => {
    const options: HTMLReactParserOptions = {
      replace(node) {
        if (!(node instanceof Element) || node.name !== "a") {
          return undefined;
        }
        const href = node.attribs.href;
        const children = domToReact(node.children as DOMNode[], options);
        if (!href) {
          return <span>{children}</span>;
        }
        return (
          <a
            href={href}
            rel="noopener noreferrer"
            onClick={(event) => {
              event.preventDefault();
              openLink(href);
            }}
            // A middle click would otherwise open the link in a new webview window.
            onAuxClick={(event) => {
              if (event.button !== 1) return;
              event.preventDefault();
              openLink(href);
            }}
          >
            {children}
          </a>
        );
      },
    };
    return parse(html, options);
  }, [html]);

  return <div className={modpackDescriptionStyles.root}>{content}</div>;
}
