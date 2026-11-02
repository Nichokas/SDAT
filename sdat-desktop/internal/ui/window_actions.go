package ui

import (
	"context"
	"log/slog"
	"strings"
	"unsafe"

	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/notifications"
	"codeberg.org/dergs/tonearm/internal/player"
	"codeberg.org/dergs/tonearm/internal/router"
	"codeberg.org/dergs/tonearm/internal/secrets"
	"codeberg.org/dergs/tonearm/internal/services/tidal/openapi"
	v2 "codeberg.org/dergs/tonearm/internal/services/tidal/v2"
	"codeberg.org/dergs/tonearm/internal/settings"
	"codeberg.org/dergs/tonearm/internal/ui/accels"
	"codeberg.org/dergs/tonearm/internal/ui/components/linking"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/tidalapi"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/auth"
	modelopenapi "codeberg.org/dergs/tonearm/pkg/tidalapi/models/openapi"
	modelv2 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v2"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gio"
	"codeberg.org/puregotk/puregotk/v4/glib"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"github.com/infinytum/injector"
)

func (w *Window) installActions() {
	aboutAction := gio.NewSimpleAction("about", nil)
	aboutAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		w.PresentAbout()
	}))
	w.GetApplication().Application.AddAction(aboutAction)

	preferencesAction := gio.NewSimpleAction("preferences", nil)
	preferencesAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		w.PresentPreferences()
	}))
	w.GetApplication().Application.AddAction(preferencesAction)
	// TIDAL uses CTRL + P for preferences
	w.GetApplication().SetAccelsForAction("app.preferences", []string{accels.PrimaryMod + "comma", accels.PrimaryMod + "p"})

	shortcutsAction := gio.NewSimpleAction("shortcuts", nil)
	shortcutsAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		w.PresentShortcuts()
	}))
	w.GetApplication().Application.AddAction(shortcutsAction)
	w.GetApplication().SetAccelsForAction("app.shortcuts", []string{accels.PrimaryMod + "question"})

	quitAction := gio.NewSimpleAction("quit", nil)
	quitAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		w.GetApplication().Quit()
	}))
	w.GetApplication().Application.AddAction(quitAction)
	w.GetApplication().SetAccelsForAction("app.quit", []string{accels.PrimaryMod + "q"})

	closeAction := gio.NewSimpleAction("close", nil)
	closeAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		w.Close()
	}))
	w.AddAction(closeAction)
	w.GetApplication().SetAccelsForAction("win.close", []string{accels.PrimaryMod + "w"})

	navigateBackAction := gio.NewSimpleAction("navigate-back", nil)
	navigateBackAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		router.Back()
	}))
	w.AddAction(navigateBackAction)
	w.GetApplication().SetAccelsForAction("win.navigate-back", []string{"<Alt>Left"})

	searchAction := gio.NewSimpleAction("search", nil)
	searchAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		router.Navigate("search")
	}))
	w.AddAction(searchAction)
	w.GetApplication().SetAccelsForAction("win.search", []string{accels.PrimaryMod + "f"})

	playTrackAction := gio.NewSimpleAction("player.play-track", glib.NewVariantType("s"))
	playTrackAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		variant := (*glib.Variant)(unsafe.Pointer(parameter))
		id := variant.GetString(nil)
		go player.PlayTrackID(id)
	}))
	w.AddAction(playTrackAction)

	playPlaylistAction := gio.NewSimpleAction("player.play-playlist", glib.NewVariantType("s"))
	playPlaylistAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		variant := (*glib.Variant)(unsafe.Pointer(parameter))
		id := variant.GetString(nil)
		go player.PlayPlaylist(id, false, 0)
	}))
	w.AddAction(playPlaylistAction)

	shuffleAction := gio.NewSimpleAction("player.shuffle", nil)
	shuffleAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		go player.ToggleShuffle()
	}))
	w.AddAction(shuffleAction)
	// TIDAL uses CTRL + S for shuffle
	w.GetApplication().SetAccelsForAction("win.player.shuffle", []string{accels.PrimaryMod + "s"})

	nextAction := gio.NewSimpleAction("player.next", nil)
	nextAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		go player.Next()
	}))
	w.AddAction(nextAction)
	// TIDAL uses CTRL + Right Arrow for next
	w.GetApplication().SetAccelsForAction("win.player.next", []string{accels.PrimaryMod + "Right"})

	playPauseAction := gio.NewSimpleAction("player.play-pause", nil)
	playPauseAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		go player.PlayPause()
	}))
	w.AddAction(playPauseAction)

	previousAction := gio.NewSimpleAction("player.previous", nil)
	previousAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		go player.Previous()
	}))
	w.AddAction(previousAction)
	// TIDAL uses CTRL + Left Arrow for previous
	w.GetApplication().SetAccelsForAction("win.player.previous", []string{accels.PrimaryMod + "Left"})

	repeatAction := gio.NewSimpleAction("player.repeat", nil)
	repeatAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		player.CycleRepeatMode()
	}))
	w.AddAction(repeatAction)
	// TIDAL uses CTRL + R for repeat
	w.GetApplication().SetAccelsForAction("win.player.repeat", []string{accels.PrimaryMod + "r"})

	queueTrackAction := gio.NewSimpleAction("player.queue-track", glib.NewVariantType("s"))
	queueTrackAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		variant := (*glib.Variant)(unsafe.Pointer(parameter))
		id := variant.GetString(nil)
		player.AddTrackToUserQueue(id)
	}))
	w.AddAction(queueTrackAction)

	queueAction := gio.NewSimpleAction("player.queue", glib.NewVariantType("s"))
	queueAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		variant := (*glib.Variant)(unsafe.Pointer(parameter))
		param := variant.GetString(nil)
		logger := slog.With("module", "window_actions", "action", "win.player.queue", "parameter", param)

		parts := strings.Split(param, "/")

		if len(parts) != 2 {
			logger.Error("parameter doesn't follow the format 'type/id'")
			return
		}

		id := parts[1]

		switch parts[0] {
		case "track":
			player.AddTrackToUserQueue(id)
		case "album":
			service := injector.MustInject[tonearm.Service]()
			go func() {
				paginator, err := service.GetAlbumTracks(id)
				if err != nil {
					logger.Error("failed to fetch album", "album_id", id, "error", err)
					return
				}

				tracks, err := paginator.GetAll()
				if err != nil {
					logger.Error("failed to fetch album", "album_id", id, "error", err)
					return
				}

				player.AddTracklistToUserQueue(tracks)
			}()
		case "playlist":
			service := injector.MustInject[tonearm.Service]()
			go func() {
				paginator, err := service.GetPlaylistTracks(id)
				if err != nil {
					logger.Error("failed to fetch playlist", "playlist_id", id, "error", err)
					return
				}

				tracks, err := paginator.GetAll()
				if err != nil {
					logger.Error("failed to fetch playlist", "playlist_id", id, "error", err)
					return
				}

				player.AddTracklistToUserQueue(tracks)
			}()
		case "artist":
			tidal := injector.MustInject[*tidalapi.TidalAPI]()
			go func() {
				artist, err := tidal.V2.Artist.Artist(context.Background(), id)
				if err != nil {
					logger.Error("failed to fetch artist", "artist_id", id, "error", err)
					return
				}

				var module modelv2.PageItem
				for _, item := range artist.Items {
					if item.ModuleID == "ARTIST_TOP_TRACKS" {
						module = item
						break
					}
				}

				var topTracks []tonearm.Track
				for _, legacyTopTrackItem := range module.Items {
					if legacyTopTrackItem.Type == modelv2.ItemTypeTrack {
						topTracks = append(topTracks, v2.NewTrack(*legacyTopTrackItem.Data.Track))
					}
				}

				player.AddTracklistToUserQueue(topTracks)
			}()
		case "my_collection":
			logger := logger.With("object_type", "my_collection").WithGroup("my_collection")
			switch parts[1] {
			case "tracks":
				tidal := injector.MustInject[*tidalapi.TidalAPI]()
				go func() {
					userId := secrets.UserID()
					if userId == "" {
						logger.Warn("user not logged in, ignoring action")
						return
					}

					paginator := openapi.NewPaginator(
						tidal.OpenAPI.V2.UserCollections.Tracks,
						userId,
						func(r *modelopenapi.Response[[]modelopenapi.Relationship]) []tonearm.Track {
							results := r.Included.Tracks(r.Data...)
							tracks := make([]tonearm.Track, len(results))
							for i, track := range results {
								tracks[i] = openapi.NewTrack(track)
							}
							return tracks
						},
						"tracks.artists",
						"tracks.albums.coverArt",
					)

					tracks, err := paginator.GetAll()
					if err != nil {
						logger.Error("failed to get tracks", "error", err)
						return
					}

					player.AddTracklistToUserQueue(tracks)
				}()
			default:
				logger.Error("unknown object type to add to queue", "type", parts[0])
				return
			}
		default:
			logger.Error("unknown object type to add to queue", "type", parts[0])
			return
		}
	}))
	w.AddAction(queueAction)

	routeHomeAction := gio.NewSimpleAction("route.home", nil)
	routeHomeAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		router.Navigate(settings.General().DefaultPage())
	}))
	w.AddAction(routeHomeAction)
	// TIDAL uses CTRL + H for route home
	w.GetApplication().SetAccelsForAction("win.route.home", []string{accels.PrimaryMod + "h"})

	routeAlbumAction := gio.NewSimpleAction("route.album", glib.NewVariantType("s"))
	routeAlbumAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		variant := (*glib.Variant)(unsafe.Pointer(parameter))
		router.Navigate("album/" + variant.GetString(nil))
	}))
	w.AddAction(routeAlbumAction)

	routePlaylistAction := gio.NewSimpleAction("route.playlist", glib.NewVariantType("s"))
	routePlaylistAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		variant := (*glib.Variant)(unsafe.Pointer(parameter))
		router.Navigate("playlist/" + variant.GetString(nil))
	}))
	w.AddAction(routePlaylistAction)

	routeArtistAction := gio.NewSimpleAction("route.artist", glib.NewVariantType("s"))
	routeArtistAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		variant := (*glib.Variant)(unsafe.Pointer(parameter))
		router.Navigate("artist/" + variant.GetString(nil))
	}))
	w.AddAction(routeArtistAction)

	signInAction := gio.NewSimpleAction("sign-in", nil)
	signInAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		go func() {
			var dialog *adw.AlertDialog
			resp, err := tidalapi.StartDeviceLinking(func(dlc *auth.DeviceLinkingChallenge, cancel context.CancelFunc) {
				schwifty.OnMainThreadOnce(func(u uintptr) {
					dialog = linking.NewLinking(&w.Window, dlc.UserCode, dlc.VerificationUriComplete, cancel)()
					dialog.Present(&w.Widget)
				}, 0)
			})
			if err != nil {
				notifications.OnToast.Notify(gettext.Get("Sign in failed or aborted"))
				logger.Error(err.Error())
				if dialog != nil {
					dialog.ForceClose()
				}
				return
			}
			defer dialog.ForceClose()
			secrets.SetRefreshToken(resp.RefreshToken)
			notifications.OnToast.Notify(gettext.Get("Signed in as %s", resp.User.Email))
			router.Refresh()
		}()
	}))
	w.AddAction(signInAction)

	signOutAction := gio.NewSimpleAction("sign-out", nil)
	signOutAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		go func() {
			secrets.DeleteRefreshToken()
			notifications.OnToast.Notify(gettext.Get("Signed out"))
			router.Refresh()
		}()
	}))
	w.AddAction(signOutAction)
	// TIDAL uses CTRL + L for sign out
	w.GetApplication().SetAccelsForAction("win.sign-out", []string{"<Primary>l"})

	setAsDefaultAction := gio.NewSimpleAction("set-as-default", nil)
	setAsDefaultAction.ConnectActivate(new(func(action gio.SimpleAction, parameter uintptr) {
		settings.General().SetDefaultPage(router.Current().Path)
		notifications.OnToast.Notify(gettext.Get("Set current page as default"))
	}))
	w.AddAction(setAsDefaultAction)
}

const (
	MouseButtonBack    = 8
	MouseButtonForward = 9
)

func (w *Window) installMouseClickHandler() {
	gestureController := gtk.NewGestureClick()
	gestureController.SetButton(0)
	gestureController.SetPropagationPhase(gtk.PhaseCaptureValue)
	gestureController.ConnectPressed(new(func(controller gtk.GestureClick, nPress int32, x float64, y float64) {
		switch controller.GetCurrentButton() {
		case MouseButtonBack:
			// Back button
			w.ActivateAction("navigate-back", nil)
		}
	}))
	w.AddController(&gestureController.EventController)
}
