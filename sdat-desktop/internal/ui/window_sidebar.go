package ui

import (
	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/router"
	"codeberg.org/dergs/tonearm/internal/secrets"
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/internal/ui/components"
	"codeberg.org/dergs/tonearm/internal/ui/components/sidebar/lyrics"
	"codeberg.org/dergs/tonearm/internal/ui/components/sidebar/player"
	"codeberg.org/dergs/tonearm/internal/ui/components/sidebar/queue"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gio"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func (w *Window) buildSidebarHeader() *gtk.Widget {
	windowTitle := WindowTitle("Tonearm", "")()
	router.Navigation.On(func(entry *router.NavigationEvent) bool {
		if entry.Completed {
			schwifty.OnMainThreadOncePure(func() {
				w.SetTitle("Tonearm - " + entry.Result.PageTitle)
			})
		}
		return signals.Continue
	})

	mainMenu := gio.NewMenu()
	mainMenu.Append(gettext.Get("Sign In…"), "win.sign-in")
	mainMenu.Append(gettext.Get("Set as Default Page"), "win.set-as-default")
	mainMenu.Append(gettext.Get("Keyboard Shortcuts"), "app.shortcuts")
	mainMenu.Append(gettext.Get("Preferences"), "app.preferences")
	mainMenu.Append(gettext.Get("About Tonearm"), "app.about")
	mainMenu.Append(gettext.Get("Quit"), "app.quit")

	secrets.SignedInChanged.On(func(signedIn bool) bool {
		mainMenu.Remove(0)
		if signedIn {
			mainMenu.Insert(0, gettext.Get("Sign Out"), "win.sign-out")
		} else {
			mainMenu.Insert(0, gettext.Get("Sign In…"), "win.sign-in")
		}
		return signals.Continue
	})

	return HeaderBar().
		TitleWidget(Widget(&windowTitle.Widget)).
		ShowBackButton(false).
		ShowEndTitleButtons(false).
		CenteringPolicy(adw.CenteringPolicyStrictValue).
		PackStart(
			components.NewRouteButton("search", false).Icon("loupe-symbolic").TooltipText(gettext.Get("Search")),
		).
		PackEnd(
			MenuButton().
				IconName("menu-symbolic").
				MenuModel(&mainMenu.MenuModel).
				TooltipText(gettext.Get("Main Menu")).ConnectConstruct(func(mb *gtk.MenuButton) {
				menuAction := gio.NewSimpleAction("main-menu", nil)
				menuAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
					mb.Popup()
				}))
				w.AddAction(menuAction)
				w.GetApplication().SetAccelsForAction("win.main-menu", []string{"F10"})
			}),
			components.NewRouteButton("feed", false).Icon("bell-outline-symbolic").TooltipText(gettext.Get("Feed")),
		).
		ToGTK()
}

func (w *Window) buildSidebar() schwifty.ViewStack {
	return ViewStack().
		AddTitledWithIcon(player.NewPlayer(), "player", gettext.Get("Player"), "music-note-outline-symbolic").
		AddTitledWithIcon(lyrics.NewLyricsPanel(), "lyrics", gettext.Get("Lyrics"), "chat-bubble-text-symbolic").
		AddTitledWithIcon(queue.NewQueue(), "queue", gettext.Get("Queue"), "music-queue-symbolic")
}

func (w *Window) buildSidebarFooter(viewStack *adw.ViewStack) *gtk.Widget {
	viewSwitcher := adw.NewViewSwitcher()
	viewSwitcher.SetPolicy(adw.ViewSwitcherPolicyWideValue)
	viewSwitcher.SetStack(viewStack)
	return &viewSwitcher.Widget
}
