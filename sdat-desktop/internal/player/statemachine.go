package player

import (
	"context"
	"log/slog"
	"reflect"
	"time"

	"codeberg.org/dergs/tonearm/internal/player/statemachine"
	"codeberg.org/dergs/tonearm/internal/settings"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"github.com/go-gst/go-gst/gst"
	"github.com/infinytum/injector"
	"github.com/qmuntal/stateless"
)

var (
	storedState     stateless.State = statemachine.STATE_STOPPED
	storedStateArgs []any
	stateMachine    = stateless.NewStateMachineWithExternalStorageAndArgs(stateAccessor, stateMutator, stateless.FiringQueued)
)

func init() {
	stateMachine.Configure(statemachine.STATE_STOPPED).
		// This happens whenever a request for a track happens that we only known the ID of.
		Permit(statemachine.TRIGGER_PLAY_TRACK_ID, statemachine.STATE_PREPARING).
		// This happens whenever a request for a track happens, where we already have all the metadata.
		Permit(statemachine.TRIGGER_PLAY_TRACK, statemachine.STATE_PREPARING).
		OnEntry(func(ctx context.Context, args ...any) error {
			return statemachine.OnStopped(playbin)
		}).
		OnEntryFrom(statemachine.TRIGGER_STOP, func(ctx context.Context, args ...any) error {
			SourceChanged.Set(nil)
			return statemachine.OnStoppedStop()
		}).
		OnEntryFrom(statemachine.TRIGGER_TOKEN_EXPIRED, func(ctx context.Context, args ...any) error {
			slog.Error("track was enqueued successfully, but playback was rejected by remote server. this may be due to a network issue, account restrictions or a temporary outage.")
			SourceChanged.Set(nil)
			return statemachine.OnStoppedStop()
		}).
		OnEntryFrom(gst.MessageEOS, func(ctx context.Context, args ...any) error {
			if !settings.Playback().AllowAutoplay() {
				SourceChanged.Set(nil)
				return statemachine.OnStoppedStop()
			}

			// Since no other songs are left in the queue, retrieve mix to play from API
			if err := PlayTrackRadio(TrackChanged.CurrentValue().ID(), true); err != nil {
				logger.Info("failed to play track radio", "error", err)
				SourceChanged.Set(nil)
				return statemachine.OnStoppedStop()
			}
			return nil
		})

	stateMachine.Configure(statemachine.STATE_PAUSED).
		// The application is permitted to transition to the PREPARING state
		// when a different track should be played by ID or reference.
		Permit(statemachine.TRIGGER_PLAY_TRACK_ID, statemachine.STATE_PREPARING).
		Permit(statemachine.TRIGGER_PLAY_TRACK, statemachine.STATE_PREPARING).
		// The application is permitted to transition to the PLAYING state
		// when playback should be resumed from the paused state.
		Permit(statemachine.TRIGGER_RESUME, statemachine.STATE_PLAYING).
		// The application is permitted to transition to the STOPPED state
		// when playback should be paused from the playing state.
		Permit(statemachine.TRIGGER_STOP, statemachine.STATE_STOPPED).
		OnEntry(func(ctx context.Context, args ...any) error {
			return statemachine.OnPaused(playbin)
		})

	stateMachine.SetTriggerParameters(statemachine.TRIGGER_PLAY_TRACK_ID, reflect.TypeFor[string](), reflect.TypeFor[bool]())
	stateMachine.SetTriggerParameters(statemachine.TRIGGER_PLAY_TRACK, reflect.TypeFor[tonearm.Track](), reflect.TypeFor[bool]())

	stateMachine.Configure(statemachine.STATE_PREPARING).
		// The application is permitted to abort playback before it has begun
		Permit(statemachine.TRIGGER_STOP, statemachine.STATE_STOPPED).
		// If the playback fails with a token expiration right after preparation,
		// we transition to the stopped state. This is likely TIDAL blocking playback.
		Permit(statemachine.TRIGGER_TOKEN_EXPIRED, statemachine.STATE_STOPPED).
		// When the stream starts, we transition to the playback state
		Permit(gst.MessageStreamStart, statemachine.STATE_PLAYING).
		OnEntry(func(ctx context.Context, args ...any) error {
			return statemachine.OnPreparing()
		}).
		OnEntryFrom(statemachine.TRIGGER_PLAY_TRACK_ID, func(ctx context.Context, args ...any) error {
			trackID := args[0].(string)
			skipHistory := args[1].(bool)
			service, err := injector.Inject[tonearm.Service]()
			if err != nil {
				return err
			}
			return statemachine.OnPlayTrackID(playbin, service, trackID, skipHistory)
		}).
		OnEntryFrom(statemachine.TRIGGER_PLAY_TRACK, func(ctx context.Context, args ...any) error {
			track := args[0].(tonearm.Track)
			skipHistory := args[1].(bool)
			service, err := injector.Inject[tonearm.Service]()
			if err != nil {
				return err
			}
			return statemachine.OnPlayTrack(playbin, service, track, skipHistory)
		})

	stateMachine.Configure(statemachine.STATE_PLAYING).
		// The application is permitted to transition to the PREPARING state
		// when a different track should be played by ID or reference.
		Permit(statemachine.TRIGGER_PLAY_TRACK_ID, statemachine.STATE_PREPARING).
		Permit(statemachine.TRIGGER_PLAY_TRACK, statemachine.STATE_PREPARING).
		// The application is permitted to transition to the PAUSED state
		// when the track should be paused.
		Permit(statemachine.TRIGGER_PAUSE, statemachine.STATE_PAUSED).
		// The application is permitted to transition to the STOPPED state
		// when the track should be stopped or the end of the track is reached.
		Permit(statemachine.TRIGGER_STOP, statemachine.STATE_STOPPED).
		Permit(gst.MessageEOS, statemachine.STATE_STOPPED).
		// The application is permitted to transition to the GAPLESS state
		// when the gapless hint is received.
		Permit(statemachine.TRIGGER_GAPLESS_HINT, statemachine.STATE_PLAYING_GAPLESS).
		// The application is permitted to transition to the RETRYING state
		// when the playback token has likely expired.
		Permit(statemachine.TRIGGER_TOKEN_EXPIRED, statemachine.STATE_PLAYING_RETRYING).
		OnEntry(func(ctx context.Context, args ...any) error {
			return statemachine.OnPlaying(playbin)
		}).
		OnExit(func(ctx context.Context, args ...any) error {
			return statemachine.OnPlayingExit()
		})

	stateMachine.Configure(statemachine.STATE_PLAYING_GAPLESS).
		// The player must be considered PLAYING even when a gapless
		// transition is in progress.
		SubstateOf(statemachine.STATE_PLAYING).
		// The application is permitted to transition to the PLAYING state
		// when the gapless transition is complete.
		Permit(gst.MessageStreamStart, statemachine.STATE_PLAYING).
		// The application is permitted to transition to the PLAYING state
		// when the gapless transition fails.
		Permit(statemachine.TRIGGER_FAILED, statemachine.STATE_PLAYING).
		OnEntry(func(ctx context.Context, args ...any) error {
			service, err := injector.Inject[tonearm.Service]()
			if err != nil {
				logger.Error("failed to enqueue next track", "error", err)
				return stateMachine.Fire(statemachine.TRIGGER_FAILED, err)
			}

			nextTrack, _, err := nextTrack(true)
			if err != nil {
				logger.Error("failed to enqueue next track", "error", err)
				return stateMachine.Fire(statemachine.TRIGGER_FAILED, err)
			}

			trackManifest, err := statemachine.OnPlayingGapless(playbin, service, nextTrack)
			if err != nil {
				logger.Error("failed to enqueue next track", "error", err)
				return stateMachine.Fire(statemachine.TRIGGER_FAILED, err)
			}

			// HACK: The state machine doesn't offer a way to store state from the entry handler
			// and pass it to the exit handler, so we use stateMutator to store what we need.
			stateMutator(ctx, statemachine.STATE_PLAYING_GAPLESS, trackManifest)

			return nil
		}).
		OnExitWith(gst.MessageStreamStart, func(ctx context.Context, args ...any) error {
			_, enqueueArgs, _ := stateMachine.StateWithArgs(ctx)
			track, didRepeat, err := nextTrack(false)
			if err != nil {
				return err
			}

			if err := statemachine.OnPlayingGaplessExit(track, enqueueArgs[0].(tonearm.TrackManifest), didRepeat); err != nil {
				return err
			}

			return nil
		})

	stateMachine.Configure(statemachine.STATE_PLAYING_RETRYING).
		// Even during a retry, the player should be considered
		// to be in the PLAYING state, as it is still playing the track.
		SubstateOf(statemachine.STATE_PLAYING).
		// When the stream starts, we should transition back to the PLAYING state.
		Permit(gst.MessageStreamStart, statemachine.STATE_PLAYING).
		// We specifically want to ignore the gapless trigger when retrying
		// as we will fire that trigger as a byproduct of the retry process.
		Ignore(statemachine.TRIGGER_GAPLESS_HINT).
		OnEntry(func(ctx context.Context, args ...any) error {
			service, err := injector.Inject[tonearm.Service]()
			if err != nil {
				return err
			}

			position, err := statemachine.OnPlayingRetry(playbin, service)
			if err != nil {
				return err
			}

			// HACK: The state machine doesn't offer a way to store state from the entry handler
			// and pass it to the exit handler, so we use stateMutator to store what we need.
			stateMutator(ctx, statemachine.STATE_PLAYING_RETRYING, position)
			return nil
		}).
		OnExit(func(ctx context.Context, args ...any) error {
			_, enqueueArgs, _ := stateMachine.StateWithArgs(ctx)
			return statemachine.OnPlayingRetryExit(playbin, enqueueArgs[0].(time.Duration))
		})

	stateMachine.OnTransitioning(func(ctx context.Context, t stateless.Transition) {
		slog.Debug("Transitioning", "source", t.Source, "destination", t.Destination, "trigger", t.Trigger)
	})
	stateMachine.OnTransitioned(func(ctx context.Context, t stateless.Transition) {
		slog.Debug("Transition completed", "source", t.Source, "destination", t.Destination, "trigger", t.Trigger)
	})
	stateMachine.OnUnhandledTrigger(func(ctx context.Context, state stateless.State, trigger stateless.Trigger, unmetGuards []string) error {
		slog.Error("Failed Transition", "state", state, "trigger", trigger, "guards", unmetGuards)
		return nil
	})
}

func stateAccessor(context.Context) (stateless.State, []any, error) {
	return storedState, storedStateArgs, nil
}

func stateMutator(ctx context.Context, state stateless.State, args ...any) error {
	storedState = state
	storedStateArgs = args
	return nil
}
