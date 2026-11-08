import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import type { JavaMajor } from "@/features/settings/model/sections";
import { samePath } from "@/features/settings/sections/java/merge-installations";

import { javaDetectDialogStyles } from "./java-detect-dialog.styles";

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
      <DialogContent className={javaDetectDialogStyles.content}>
        <DialogTitle>{row.label}</DialogTitle>
        <div className={javaDetectDialogStyles.tableWrapper}>
          <table className={javaDetectDialogStyles.table}>
            <thead className={javaDetectDialogStyles.tableHead}>
              <tr>
                <th className={javaDetectDialogStyles.headerCell} scope="col">
                  Versão
                </th>
                <th className={javaDetectDialogStyles.headerCell} scope="col">
                  Caminho
                </th>
                <th
                  className={javaDetectDialogStyles.lastHeaderCell}
                  scope="col"
                >
                  Ações
                </th>
              </tr>
            </thead>
            <tbody>
              {installations.map((path) => {
                const selected = samePath(path, value);
                return (
                  <tr key={path} className={javaDetectDialogStyles.row}>
                    <td className={javaDetectDialogStyles.cell}>{row.major}</td>
                    <td className={javaDetectDialogStyles.pathCell}>{path}</td>
                    <td className={javaDetectDialogStyles.lastCell}>
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
