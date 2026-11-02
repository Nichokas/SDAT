package player

import (
	"fmt"

	"codeberg.org/dergs/tonearm/internal/player/playqueue"
	"codeberg.org/dergs/tonearm/internal/player/statemachine"
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"github.com/infinytum/injector"
)

var (
	BaseQueue = playqueue.NewDurableQueue()
	UserQueue = playqueue.NewQueue()
	History   = playqueue.NewHistoryQueue()
)

func init() {
	statemachine.TrackChanged.On(func(event *statemachine.TrackChangedEvent) bool {
		if event != nil && !event.SkipHistory {
			History.Prepend(event)
		}
		return signals.Continue
	})
}

func clearQueues() {
	UserQueue.Clear()
	BaseQueue.Clear()
}

func getNextTrackFromQueue(peek bool) (nextTrack tonearm.Track) {
	verb := "pop"
	if peek {
		verb = "peek"
	}
	logger.Debug(fmt.Sprintf("attempting to %s next track from user queue", verb))
	if peek {
		nextTrack = UserQueue.Peek()
	} else {
		nextTrack = UserQueue.Pop()
	}
	if nextTrack != nil {
		logger.Info(fmt.Sprintf("%sed next track from user queue", verb), "track_id", nextTrack.ID())
		return nextTrack
	}

	logger.Debug(fmt.Sprintf("attempting to %s next track from base queue", verb))
	if peek {
		nextTrack = BaseQueue.Peek()
	} else {
		nextTrack = BaseQueue.Pop()
	}
	if nextTrack != nil {
		logger.Info(fmt.Sprintf("%sed next track from base queue", verb), "track_id", nextTrack.ID())
		return nextTrack
	}

	// Check if we are suposed to repeat the base queue
	if RepeatModeChanged.CurrentValue() == RepeatModeQueue {
		logger.Debug("queue repeat mode is enabled, replaying base queue")
		BaseQueue.Restore("")
		if ShuffleStateChanged.CurrentValue() {
			BaseQueue.Shuffle("")
		}
		return BaseQueue.Peek()
	}

	return nil
}

func AddTrackToUserQueue(trackId string) error {
	service, err := injector.Inject[tonearm.Service]()
	if err != nil {
		return err
	}

	track, err := service.GetTrack(trackId)
	if err != nil {
		return err
	}
	UserQueue.Append(track)

	// If we added a song to the queue and nothing is playing, the user likely wants to start playing the queue
	if PlaybackStateChanged.CurrentValue().Status == PlaybackStatusStopped {
		logger.Info("no track is currently playing, immediately playing track", "track_id", trackId)
		Next()
		return nil
	}
	return nil
}

func AddTracklistToUserQueue(tracklist []tonearm.Track) error {
	for _, track := range tracklist {
		UserQueue.Append(track)
	}

	// If we added a song to the queue and nothing is playing, the user likely wants to start playing the queue
	if PlaybackStateChanged.CurrentValue().Status == PlaybackStatusStopped && len(tracklist) > 0 {
		logger.Info("no track is currently playing, immediately playing track", "track_id", tracklist[0].ID())
		Next()
		return nil
	}
	return nil
}

func SkipThroughQueue(queue playqueue.Queue, to int) {
	go func() {
		if skipped, err := queue.Skip(to); err == nil {
			for _, track := range skipped {
				History.Prepend(track)
			}
			if err := stateMachine.Fire(statemachine.TRIGGER_PLAY_TRACK, queue.Pop(), false); err != nil {
				logger.Error("failed to start playback after skipping through queue", "error", err)
			}
		} else {
			logger.Error("failed to skip through queue", "error", err)
		}
	}()
}
