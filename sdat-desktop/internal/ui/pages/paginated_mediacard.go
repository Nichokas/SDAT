package pages

import (
	"log/slog"

	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/internal/ui/components"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/pagination"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func NewPaginatedMediaCardPage[T any](
	paginator *pagination.Paginator[T],
	factory func(T) schwifty.BaseWidgetable,
) (schwifty.BaseWidgetable, error) {
	firstPage, err := paginator.GetFirstPage()
	if err != nil {
		return nil, err
	}

	list := WrapBox().VMargin(20).HMargin(40).VAlign(gtk.AlignStartValue).Justify(adw.JustifyFillValue).JustifyLastLine(true)()
	for _, item := range firstPage {
		child := CenterBox().CenterWidget(factory(item)).ToGTK()
		list.Append(child)
	}
	listRef := weak.NewWidgetRef(list)

	return ScrolledWindow().
		Child(
			components.MainContent(Widget(&list.Widget)),
		).
		Policy(gtk.PolicyNeverValue, gtk.PolicyAutomaticValue).
		ConnectReachEdgeSoon(gtk.PosBottomValue, func() bool {
			if !paginator.IsConsumed() {
				items, err := paginator.Next()
				if err != nil {
					return signals.Continue
				}

				schwifty.OnMainThreadOncePure(func() {
					listRef.Use(func(obj *gtk.Widget) {
						list := adw.WrapBoxNewFromInternalPtr(obj.Ptr)
						for _, item := range items {
							child := CenterBox().CenterWidget(factory(item)).ToGTK()
							list.Append(child)
						}
					})
				})
			} else {
				slog.Debug("No more items to fetch")
				return signals.Unsubscribe
			}
			return signals.Continue
		}), nil
}
