package syntax

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/bindings"
	adwbindings "codeberg.org/dergs/tonearm/pkg/schwifty/bindings/adw"
	gtkbindings "codeberg.org/dergs/tonearm/pkg/schwifty/bindings/gtk"
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func ActionRow() adwbindings.ActionRow {
	return managedWidget("ActionRow", func() *adw.ActionRow {
		actionRow := adw.NewActionRow()
		actionRow.ConnectActivated(&callback.ActionRowActivated)
		return actionRow
	})
}

func AlertDialog(heading string, body string) adwbindings.AlertDialog {
	return managedWidget("AlertDialog", func() *adw.AlertDialog {
		dialog := adw.NewAlertDialog(heading, body)
		dialog.ConnectClosed(&callback.AlertDialogClosed)
		dialog.ConnectCloseAttempt(&callback.AlertDialogCloseAttempt)
		dialog.ConnectResponse(&callback.AlertDialogResponse)
		return dialog
	})
}

func Bin() adwbindings.Bin {
	return managedWidget("Bin", func() *adw.Bin {
		return adw.NewBin()
	})
}

func ButtonRow() adwbindings.ButtonRow {
	return managedWidget("ButtonRow", func() *adw.ButtonRow {
		return adw.NewButtonRow()
	})
}

func Clamp() adwbindings.Clamp {
	return managedWidget("Clamp", func() *adw.Clamp {
		return adw.NewClamp()
	})
}

func ClampScrollable() adwbindings.ClampScrollable {
	return managedWidget("ClampScrollable", func() *adw.ClampScrollable {
		return adw.NewClampScrollable()
	})
}

func ComboRow() adwbindings.ComboRow {
	return managedWidget("ComboRow", func() *adw.ComboRow {
		comboRow := adw.NewComboRow()
		comboRow.ConnectNotify(&adwbindings.ComboRowSelectionChangedCallback)
		return comboRow
	})
}

func ExpanderRow(rows ...any) adwbindings.ExpanderRow {
	return managedWidget("ExpanderRow", func() *adw.ExpanderRow {
		expanderRow := adw.NewExpanderRow()
		for _, row := range rows {
			expanderRow.AddRow(gtkbindings.ResolveWidget(row))
		}
		return expanderRow
	})
}

func EntryRow() adwbindings.EntryRow {
	return managedWidget("EntryRow", func() *adw.EntryRow {
		return adw.NewEntryRow()
	})
}

func HeaderBar() adwbindings.HeaderBar {
	return managedWidget("HeaderBar", func() *adw.HeaderBar {
		return adw.NewHeaderBar()
	})
}

func PasswordEntryRow() adwbindings.PasswordEntryRow {
	return managedWidget("PasswordEntryRow", func() *adw.PasswordEntryRow {
		return adw.NewPasswordEntryRow()
	})
}

func PreferencesDialog(pages ...any) adwbindings.PreferencesDialog {
	return managedWidget("PreferencesDialog", func() *adw.PreferencesDialog {
		dialog := adw.NewPreferencesDialog()
		for _, page := range pages {
			dialog.Add(bindings.ResolveTo[*adw.PreferencesPage, adwbindings.PreferencesPage](page))
		}
		return dialog
	})
}

func PreferencesGroup(children ...any) adwbindings.PreferencesGroup {
	return managedWidget("PreferencesGroup", func() *adw.PreferencesGroup {
		group := adw.NewPreferencesGroup()
		for _, child := range children {
			group.Add(gtkbindings.ResolveWidget(child))
		}
		return group
	})
}

func PreferencesPage(groups ...any) adwbindings.PreferencesPage {
	return managedWidget("PreferencesPage", func() *adw.PreferencesPage {
		page := adw.NewPreferencesPage()
		for _, group := range groups {
			page.Add(bindings.ResolveTo[*adw.PreferencesGroup, adwbindings.PreferencesGroup](group))
		}
		return page
	})
}

func ShortcutsDialog(sections ...any) adwbindings.ShortcutsDialog {
	return managedWidget("ShortcutsDialog", func() *adw.ShortcutsDialog {
		dialog := adw.NewShortcutsDialog()
		for _, section := range sections {
			dialog.Add(bindings.ResolveTo[*adw.ShortcutsSection, adwbindings.ShortcutsSection](section))
		}
		return dialog
	})
}

func ShortcutsItem(title string, accelerator string) adwbindings.ShortcutsItem {
	return managedObject("ShortcutsItem", func() *adw.ShortcutsItem {
		group := adw.NewShortcutsItem(title, accelerator)
		return group
	})
}

func ShortcutsItemFromAction(title string, action string) adwbindings.ShortcutsItem {
	return managedObject("ShortcutsItem", func() *adw.ShortcutsItem {
		group := adw.NewShortcutsItemFromAction(title, action)
		return group
	})
}

func ShortcutsSection(items ...any) adwbindings.ShortcutsSection {
	return managedObject("ShortcutsSection", func() *adw.ShortcutsSection {
		group := adw.NewShortcutsSection("")
		for _, item := range items {
			group.Add(bindings.ResolveTo[*adw.ShortcutsItem, adwbindings.ShortcutsItem](item))
		}
		return group
	})
}

func Spinner() adwbindings.Spinner {
	return managedWidget("Spinner", func() *adw.Spinner {
		return adw.NewSpinner()
	})
}

func SpinRow(adjustment *gtk.Adjustment, climbRate float64, digits uint32) adwbindings.SpinRow {
	return managedWidget("SpinRow", func() *adw.SpinRow {
		return adw.NewSpinRow(adjustment, climbRate, digits)
	})
}

func StatusPage() adwbindings.StatusPage {
	return managedWidget("StatusPage", func() *adw.StatusPage {
		return adw.NewStatusPage()
	})
}

func SwitchRow() adwbindings.SwitchRow {
	return managedWidget("SwitchRow", func() *adw.SwitchRow {
		return adw.NewSwitchRow()
	})
}

func ViewStack(children ...any) adwbindings.ViewStack {
	return managedWidget("ViewStack", func() *adw.ViewStack {
		viewStack := adw.NewViewStack()
		for _, child := range children {
			viewStack.Add(gtkbindings.ResolveWidget(child))
		}
		return viewStack
	})
}

func WindowTitle(title string, subtitle string) adwbindings.WindowTitle {
	return managedWidget("WindowTitle", func() *adw.WindowTitle {
		return adw.NewWindowTitle(title, subtitle)
	})
}

func WrapBox(children ...any) adwbindings.WrapBox {
	return managedWidget("WrapBox", func() *adw.WrapBox {
		box := adw.NewWrapBox()
		for _, child := range children {
			box.Append(gtkbindings.ResolveWidget(child))
		}
		return box
	})
}
