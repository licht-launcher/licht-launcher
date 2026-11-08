import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import type { JavaMajor } from "@/features/settings/model/sections";
import { samePath } from "@/features/settings/sections/java/merge-installations";

export function JavaDetectDialog({
  open,
  onOpenChange,
  row,
  value,
  installations,
  onSelect,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  row: JavaMajor;
  value: string;
  installations: string[];
  onSelect: (path: string) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-3xl">
        <DialogTitle>{row.label}</DialogTitle>
        <div className="overflow-x-auto">
          <table className="w-full text-left text-sm">
            <thead className="text-muted-foreground">
              <tr>
                <th className="pr-3 font-medium" scope="col">
                  Versão
                </th>
                <th className="pr-3 font-medium" scope="col">
                  Caminho
                </th>
                <th className="font-medium" scope="col">
                  Ações
                </th>
              </tr>
            </thead>
            <tbody>
              {installations.map((path) => {
                const selected = samePath(path, value);
                return (
                  <tr key={path} className="border-t border-border">
                    <td className="py-2 pr-3">{row.major}</td>
                    <td className="py-2 pr-3 break-all">{path}</td>
                    <td className="py-2">
                      <Button
                        type="button"
                        variant="outline"
                        size="default"
                        disabled={selected}
                        onClick={() => onSelect(path)}
                      >
                        {selected ? "Selecionado" : "Selecionar"}
                      </Button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </DialogContent>
    </Dialog>
  );
}
