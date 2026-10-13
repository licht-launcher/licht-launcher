import { cn } from "cn";
import * as ResizablePrimitive from "react-resizable-panels";

function ResizablePanelGroup({
  className,
  ...props
}: ResizablePrimitive.GroupProps) {
  return (
    <ResizablePrimitive.Group
      data-slot="resizable-panel-group"
      className={cn(
        "flex h-full w-full aria-[orientation=vertical]:flex-col",
        className,
      )}
      {...props}
    />
  );
}

function ResizablePanel({ ...props }: ResizablePrimitive.PanelProps) {
  return <ResizablePrimitive.Panel data-slot="resizable-panel" {...props} />;
}

function ResizableHandle({
  withHandle,
  className,
  ...props
}: ResizablePrimitive.SeparatorProps & {
  withHandle?: boolean;
}) {
  return (
    <ResizablePrimitive.Separator
      data-slot="resizable-handle"
      className={cn(
        "relative flex w-1 shrink-0 cursor-ew-resize items-center justify-center bg-transparent transition-colors duration-150 ring-offset-background after:absolute after:inset-y-0 after:left-1/2 after:w-px after:-translate-x-1/2 after:bg-border after:transition-[width,background-color] after:duration-150 hover:after:w-0.5 hover:after:bg-muted-foreground/40 focus-visible:ring-1 focus-visible:ring-ring focus-visible:outline-hidden focus-visible:after:w-0.5 focus-visible:after:bg-muted-foreground/40 aria-[orientation=horizontal]:h-1 aria-[orientation=horizontal]:w-full aria-[orientation=horizontal]:cursor-ns-resize aria-[orientation=horizontal]:after:inset-x-0 aria-[orientation=horizontal]:after:top-1/2 aria-[orientation=horizontal]:after:left-0 aria-[orientation=horizontal]:after:h-px aria-[orientation=horizontal]:after:w-full aria-[orientation=horizontal]:after:translate-x-0 aria-[orientation=horizontal]:after:-translate-y-1/2 aria-[orientation=horizontal]:hover:after:h-0.5 aria-[orientation=horizontal]:hover:after:w-full aria-[orientation=horizontal]:focus-visible:after:h-0.5 aria-[orientation=horizontal]:focus-visible:after:w-full [&[aria-orientation=horizontal]>div]:rotate-90",
        className,
      )}
      {...props}
    >
      {withHandle ? (
        <div className="z-10 flex h-6 w-1 shrink-0 rounded-lg bg-border" />
      ) : null}
    </ResizablePrimitive.Separator>
  );
}

export { ResizableHandle, ResizablePanel, ResizablePanelGroup };
