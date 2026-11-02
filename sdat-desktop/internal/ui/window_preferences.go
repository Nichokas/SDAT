package ui

import (
	"fmt"
	"log/slog"

	"codeberg.org/dergs/tonearm/internal/features/scrobbling"
	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/settings"
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	adwbindings "codeberg.org/dergs/tonearm/pkg/schwifty/bindings/adw"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func buildPreferencesGeneral(dialog *adw.PreferencesDialog) adwbindings.PreferencesPage {
	return PreferencesPage(
		PreferencesGroup(
			SwitchRow().
				Title(gettext.Get("Allow Background Activity")).
				Subtitle(gettext.Get("Allow Tonearm to run in the background by hiding the player window instead of quitting the application")).
				ConnectConstruct(func(sr *adw.SwitchRow) {
					settings.General().BindRunInBackground(&sr.Object, "active")
				}),
		).
			Title(gettext.Get("Background Activity")).
			Description(gettext.Get("Configure the behaviour of Tonearm when running in the background")),
		PreferencesGroup(
			EntryRow().
				Title(gettext.Get("Default Page")).
				ConnectConstruct(func(sr *adw.EntryRow) {
					settings.General().BindDefaultPage(&sr.Object, "text")
				}),
			SpinRow(
				gtk.NewAdjustment(0, 1, 100, 1, 1, 1),
				1,
				0,
			).Title(gettext.Get("History Length")).
				Subtitle(gettext.Get("Maximum history length before dropping old entries")).
				ConnectConstruct(func(sr *adw.SpinRow) {
					settings.Performance().BindMaxRouterHistorySize(&sr.Object, "value")
				}),
		).
			Title(gettext.Get("Navigation Behaviour")).
			Description(gettext.Get("Configure the behaviour of Tonearm when navigating between pages")),
		PreferencesGroup(
			ExpanderRow(
				EntryRow().
					Title(gettext.Get("API Base URL")).
					ConnectConstruct(func(sr *adw.EntryRow) {
						settings.ServiceTidal().BindAPIBaseURL(&sr.Object, "text")
					}),
				EntryRow().
					Title(gettext.Get("Auth Base URL")).
					ConnectConstruct(func(sr *adw.EntryRow) {
						settings.ServiceTidal().BindAuthBaseURL(&sr.Object, "text")
					}),
				EntryRow().
					Title(gettext.Get("OpenAPI Base URL")).
					ConnectConstruct(func(sr *adw.EntryRow) {
						settings.ServiceTidal().BindOpenAPIBaseURL(&sr.Object, "text")
					}),
				EntryRow().
					Title(gettext.Get("Resources Base URL")).
					ConnectConstruct(func(sr *adw.EntryRow) {
						settings.ServiceTidal().BindResourcesBaseURL(&sr.Object, "text")
					}),
			).
				Title(gettext.Get("Base URLs")).
				Subtitle(gettext.Get("Configure the base URLs for TIDAL")),
		).
			Title(gettext.Get("TIDAL Settings")).
			Description(gettext.Get("TIDAL-specific settings for advanced users")),
	).Title(gettext.Get("General")).IconName("settings-symbolic")
}

func buildPreferencesPlayback(*adw.PreferencesDialog) adwbindings.PreferencesPage {
	return PreferencesPage(
		PreferencesGroup(
			SwitchRow().
				Title(gettext.Get("Enable Autoplay")).
				Subtitle(gettext.Get("Allow Tonearm to start a mix with the current playing song when you're at the end of an album/queue")).
				ConnectConstruct(func(sr *adw.SwitchRow) {
					settings.Playback().BindAllowAutoplay(&sr.Object, "active")
				}),
			SwitchRow().
				Title(gettext.Get("Normalize Volume")).
				Subtitle(gettext.Get("Set the same volume for all tracks")).
				ConnectConstruct(func(sr *adw.SwitchRow) {
					settings.Playback().BindNormalizeVolume(&sr.Object, "active")
				}),
			ComboRow().
				Title(gettext.Get("Preferred Replay Gain")).
				Subtitle(gettext.Get("Choose whether Tonearm should prefer album replay gain, track replay gain or decide automatically")).
				Model(gtk.NewStringList(settings.ReplayGainModeStrings())).
				Selected(uint32(settings.Playback().ReplayGainMode())).
				ConnectSelectionChanged(func(a uint32) {
					settings.Playback().SetReplayGainMode(settings.ReplayGainMode(a))
				}),
		).Title(gettext.Get("Playback")).Description(gettext.Get("Configure the behaviour of Tonearm regarding playback")),
	).Title(gettext.Get("Playback")).IconName("media-playback-start-symbolic")
}

func buildPreferencesPerformance(*adw.PreferencesDialog) adwbindings.PreferencesPage {
	return PreferencesPage(
		PreferencesGroup(
			SwitchRow().
				Title(gettext.Get("Allow Media Card Images")).
				Subtitle(gettext.Get("Allow Tonearm to load images for media card buttons")).
				ConnectConstruct(func(sr *adw.SwitchRow) {
					settings.Performance().BindAllowMediaCardImages(&sr.Object, "active")
				}),
			SwitchRow().
				Title(gettext.Get("Allow Shortcut Images")).
				Subtitle(gettext.Get("Allow Tonearm to load images for shortcut buttons")).
				ConnectConstruct(func(sr *adw.SwitchRow) {
					settings.Performance().BindAllowShortcutImages(&sr.Object, "active")
				}),
			SwitchRow().
				Title(gettext.Get("Allow Tracklist Images")).
				Subtitle(gettext.Get("Allow Tonearm to load images for tracklists with the cover column")).
				ConnectConstruct(func(sr *adw.SwitchRow) {
					settings.Performance().BindAllowTracklistImages(&sr.Object, "active")
				}),
			SwitchRow().
				Title(gettext.Get("Cache Images")).
				Subtitle(gettext.Get("Allow Tonearm to retain recently loaded images in memory to improve performance and reduce network traffic (estimated 50 MiB limit)")).
				ConnectConstruct(func(sr *adw.SwitchRow) {
					settings.Performance().BindCacheImages(&sr.Object, "active")
				}),
		).Title(gettext.Get("Images")).Description(gettext.Get("Configure the behaviour of Tonearm regarding images")),
	).Title(gettext.Get("Performance")).IconName("speedometer5-symbolic")
}

func buildPreferencesScrobbling(dialog *adw.PreferencesDialog) adwbindings.PreferencesPage {
	dialogRef := weak.NewWidgetRef(dialog)
	isLastFmLoggedIn := signals.NewStatefulSignal(settings.Scrobbling().LastFMToken() != "")

	var lastFmLoggedInSubscription *signals.Subscription
	return PreferencesPage(
		PreferencesGroup(
			SwitchRow().
				Title(gettext.Get("Enable ListenBrainz")).
				Subtitle(gettext.Get("Allow Tonearm to send scrobbling data to ListenBrainz")).
				ConnectConstruct(func(sr *adw.SwitchRow) {
					settings.Scrobbling().BindEnableListenBrainz(&sr.Object, "active")
				}),
			PasswordEntryRow().
				Title(gettext.Get("ListenBrainz API Token")).
				ConnectConstruct(func(sr *adw.PasswordEntryRow) {
					settings.Scrobbling().BindListenBrainzToken(&sr.Object, "text")
				}),
			EntryRow().
				Title(gettext.Get("ListenBrainz API URL")).
				ConnectConstruct(func(sr *adw.EntryRow) {
					settings.Scrobbling().BindListenBrainzUrl(&sr.Object, "text")
				}),
		).
			Title("ListenBrainz").
			Description(gettext.Get("Configure Tonearm to send scrobbling data to ListenBrainz")),
		PreferencesGroup(
			SwitchRow().
				Title(gettext.Get("Enable Last.fm")).
				Subtitle(gettext.Get("Allow Tonearm to send scrobbling data to Last.fm")).
				ConnectConstruct(func(sr *adw.SwitchRow) {
					settings.Scrobbling().BindEnableLastFM(&sr.Object, "active")
				}),
			ActionRow().
				Title(gettext.Get("Log In to Last.fm…")).
				Subtitle(gettext.Get("You are currently not logged in to Last.fm")).
				ConnectUnrealize(func(w gtk.Widget) {
					isLastFmLoggedIn.Unsubscribe(lastFmLoggedInSubscription)
				}).
				ConnectRealize(func(widget gtk.Widget) {
					actionRowRef := weak.NewWidgetRef(&widget)
					lastFmLoggedInSubscription = isLastFmLoggedIn.On(func(b bool) bool {
						if b {
							actionRowRef.Use(func(obj *gtk.Widget) {
								ar := adw.ActionRowNewFromInternalPtr(obj.Ptr)
								ar.SetTitle(gettext.Get("Log Out of Last.fm"))

								if user, err := scrobbling.LastFmScrobbler.Client.User.SelfInfo(); err != nil {
									slog.Error("error while fetching Last.fm user", "error", err, "component", "window_preferences")
									ar.SetSubtitle(gettext.Get("You are logged in to Last.fm"))
								} else {
									ar.SetSubtitle(fmt.Sprintf(
										gettext.Get("You are logged in to Last.fm as %s"),
										user.Name,
									))
								}
							})
						} else {
							actionRowRef.Use(func(obj *gtk.Widget) {
								ar := adw.ActionRowNewFromInternalPtr(obj.Ptr)
								ar.SetTitle(gettext.Get("Log In to Last.fm…"))
								ar.SetSubtitle(gettext.Get("You are currently not logged in to Last.fm"))
							})
						}

						return signals.Continue
					})
				}).
				ActionSuffix(
					Button().
						IconName("key-login-symbolic").
						ConnectConstruct(func(b *gtk.Button) {
							buttonRef := weak.NewWidgetRef(b)

							isLastFmLoggedIn.On(func(loggedIn bool) bool {
								if loggedIn {
									schwifty.OnMainThreadOncePure(func() {
										buttonRef.Use(func(obj *gtk.Widget) {
											button := gtk.ButtonNewFromInternalPtr(obj.Ptr)
											button.SetIconName("system-log-out-symbolic")
										})
									})
								} else {
									schwifty.OnMainThreadOncePure(func() {
										buttonRef.Use(func(obj *gtk.Widget) {
											button := gtk.ButtonNewFromInternalPtr(obj.Ptr)
											button.SetIconName("key-login-symbolic")
										})
									})
								}

								return signals.Continue
							})
						}).
						ConnectClicked(func(gtk.Button) {
							if isLastFmLoggedIn.CurrentValue() {
								go func() {
									if err := scrobbling.LastFmScrobbler.Unconfigure(); err != nil {
										schwifty.OnMainThreadOncePure(func() {
											dialogRef.Use(func(obj *gtk.Widget) {
												dialog := adw.PreferencesDialogNewFromInternalPtr(obj.Ptr)

												toast := adw.NewToast(gettext.Get("An error occurred while logging out of Last.fm"))
												toast.SetTimeout(3)
												dialog.AddToast(toast)
											})
										})
									} else {
										isLastFmLoggedIn.Set(false)
										schwifty.OnMainThreadOncePure(func() {
											dialogRef.Use(func(obj *gtk.Widget) {
												dialog := adw.PreferencesDialogNewFromInternalPtr(obj.Ptr)

												toast := adw.NewToast(gettext.Get("Logged out of Last.fm"))
												toast.SetTimeout(3)
												dialog.AddToast(toast)
											})
										})
									}
								}()
							} else {
								go func() {
									if completed, err := scrobbling.LastFmScrobbler.Configure(); err != nil {
										schwifty.OnMainThreadOncePure(func() {
											dialogRef.Use(func(obj *gtk.Widget) {
												dialog := adw.PreferencesDialogNewFromInternalPtr(obj.Ptr)

												toast := adw.NewToast(gettext.Get("An error occurred while logging in to Last.fm"))
												toast.SetTimeout(3)
												dialog.AddToast(toast)
											})
										})
									} else if completed {
										isLastFmLoggedIn.Set(true)
										schwifty.OnMainThreadOncePure(func() {
											dialogRef.Use(func(obj *gtk.Widget) {
												user, err := scrobbling.LastFmScrobbler.Client.User.SelfInfo()
												if err != nil {
													return
												}

												dialog := adw.PreferencesDialogNewFromInternalPtr(obj.Ptr)

												toast := adw.NewToast(fmt.Sprintf(gettext.Get("Logged in to Last.fm as %s"), user.Name))
												toast.SetTimeout(3)
												dialog.AddToast(toast)
											})
										})
									}
								}()
							}
						}).
						VAlign(gtk.AlignCenterValue),
				),
		).
			Title("Last.fm").
			Description(gettext.Get("Configure Tonearm to send scrobbling data to Last.fm")),
	).Title(gettext.Get("Scrobbling")).IconName("podcast-symbolic")
}

func (w *Window) PresentPreferences() {
	dialog := PreferencesDialog()()

	dialog.Add(buildPreferencesGeneral(dialog)())
	dialog.Add(buildPreferencesPlayback(dialog)())
	dialog.Add(buildPreferencesPerformance(dialog)())
	dialog.Add(buildPreferencesScrobbling(dialog)())

	dialog.Present(&w.Widget)
}
