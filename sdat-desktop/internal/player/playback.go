package player

import (
	"context"
	"fmt"
	"strconv"

	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/notifications"
	"codeberg.org/dergs/tonearm/internal/player/statemachine"
	"codeberg.org/dergs/tonearm/pkg/tidalapi"
	v2 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v2"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"github.com/infinytum/injector"
)

func PlayTrackID(trackID string) error {
	service, err := injector.Inject[tonearm.Service]()
	if err != nil {
		return err
	}

	track, err := service.GetTrack(trackID)
	if err != nil {
		return err
	}

	return PlayTrack(track)
}

func PlayTrack(track tonearm.Track) error {
	clearQueues()
	Stop()
	SourceChanged.Set(track)
	return stateMachine.Fire(statemachine.TRIGGER_PLAY_TRACK, track, false)
}

func PlayTracklist(source tonearm.PlaybackSource, tracklist []tonearm.Track, shuffle bool, position int) error {
	if _, err := playTracklist(tonearm.NewArrayPaginator(tracklist), shuffle, position); err != nil {
		return err
	}

	SourceChanged.Set(source)
	return nil
}

func PlayAlbum(albumId string, shuffle bool, position int) error {
	service, err := injector.Inject[tonearm.Service]()
	if err != nil {
		return err
	}

	albumInfo, err := service.GetAlbumInfo(albumId)
	if err != nil {
		return err
	}

	paginator, err := service.GetAlbumTracks(albumId)
	if err != nil {
		return err
	}

	if _, err := playTracklist(paginator, shuffle, position); err != nil {
		return err
	}

	SourceChanged.Set(albumInfo)

	return nil
}

func PlayArtistTopSongs(artistId string, shuffle bool, position int) error {
	tidal, err := injector.Inject[*tidalapi.TidalAPI]()
	if err != nil {
		return err
	}

	service, err := injector.Inject[tonearm.Service]()
	if err != nil {
		return err
	}

	artistInfo, err := service.GetArtistInfo(artistId)
	if err != nil {
		return err
	}

	artist, err := tidal.V2.Artist.Artist(context.Background(), artistId)
	if err != nil {
		return err
	}

	var module v2.PageItem
	for _, item := range artist.Items {
		if item.ModuleID == "ARTIST_TOP_TRACKS" {
			module = item
			break
		}
	}

	var topTracks []tonearm.Track

	for _, legacyTopTrackItem := range module.Items {
		if legacyTopTrackItem.Type == v2.ItemTypeTrack {
			topTrack, err := service.GetTrack(strconv.Itoa(legacyTopTrackItem.Data.Track.ID))
			if err != nil {
				logger.Error("error while resolving Top Track item", "track_id", legacyTopTrackItem.Data.Track.ID, "message", err.Error())
				continue
			}

			topTracks = append(topTracks, topTrack)
		}
	}
	if _, err := playTracklist(tonearm.NewArrayPaginator(topTracks), shuffle, position); err != nil {
		return err
	}

	if err == nil {
		SourceChanged.Set(artistInfo)
	}

	return err
}

func PlayPlaylist(playlistId string, shuffle bool, position int) error {
	service, err := injector.Inject[tonearm.Service]()
	if err != nil {
		return err
	}

	playlistInfo, err := service.GetPlaylistInfo(playlistId)
	if err != nil {
		return err
	}

	paginator, err := service.GetPlaylistTracks(playlistId)
	if err != nil {
		return err
	}

	if _, err := playTracklist(paginator, shuffle, position); err != nil {
		return err
	}

	SourceChanged.Set(playlistInfo)

	return nil
}

func PlayTrackRadio(trackId string, skipSelf bool) error {
	service, err := injector.Inject[tonearm.Service]()
	if err != nil {
		return err
	}

	playlist, err := service.GetTrackRadio(trackId)
	if err != nil {
		logger.Error("failed to retrieve mix", "error", err)
		return err
	}

	if playlist == nil {
		notifications.OnToast.Notify(gettext.Get("No mix found for the current track"))
		return fmt.Errorf("no mix found for the current track")
	}

	position := 0
	if skipSelf {
		position = 1
	}

	return PlayPlaylist(playlist.ID(), false, position)
}

func playTracklist(paginator tonearm.Paginator[tonearm.Track], shuffle bool, startingPosition int) (tonearm.Track, error) {
	// When a tracklist change is initiated, all queues are cleared
	clearQueues()

	// We want to notify everyone that we have stopped playing whatever was currently playing
	// as the current queue has just been invalidated
	Stop()

	tracks, err := paginator.GetAll()
	if err != nil {
		return nil, err
	}

	// Now we want to register the given tracklist as the base queue
	BaseQueue.Set(tracks)

	// If the user wants to play the tracklist in shuffle mode, we shuffle the queue
	// and enable the shuffle mode for the player.
	if shuffle {
		SetShuffle(true, "")
	} else {
		SetShuffle(false, "")
	}

	// If the user wants to start playing the tracklist at an offset
	// we skip the queue to the desired position
	if startingPosition > 0 {
		BaseQueue.Skip(startingPosition)
	}

	// Now we need to kick-off playback by fetching the next track from the queue
	// and starting playback.
	nextTrack, _, err := nextTrack(false)
	if err != nil {
		return nil, err
	}

	if nextTrack == nil {
		return nil, fmt.Errorf("tracklist produced no valid tracks to be played")
	}

	if err := stateMachine.Fire(statemachine.TRIGGER_PLAY_TRACK, nextTrack, false); err != nil {
		return nil, err
	}

	return nextTrack, nil
}
