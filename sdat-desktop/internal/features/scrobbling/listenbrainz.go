package scrobbling

import (
	"bytes"
	"encoding/json"
	"io"
	"net/http"
	"strings"

	"codeberg.org/dergs/tonearm/internal/settings"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

type listenBrainzListenType string

const (
	listenBrainzListenTypePlayingNow listenBrainzListenType = "playing_now"
	listenBrainzListenTypeSingle     listenBrainzListenType = "single"
)

type ListenBrainzScrobblerType struct{}

var ListenBrainzScrobbler = ListenBrainzScrobblerType{}

func init() {
	Scrobblers = append(Scrobblers, &ListenBrainzScrobbler)
}

func (scrobbler *ListenBrainzScrobblerType) NowPlaying(track tonearm.Track) {
	scrobbler.makeRequest(scrobbler.generateRequest(track, listenBrainzListenTypePlayingNow))
}

func (scrobbler *ListenBrainzScrobblerType) Scrobble(event *ScrobbleEvent) {
	req := scrobbler.generateRequest(event.Track, listenBrainzListenTypeSingle)
	req.Payload[0].ListenedAt = event.ListenedAt.Unix()
	scrobbler.makeRequest(req)
}

func (*ListenBrainzScrobblerType) makeRequest(reqBody listenBrainzRequest) {
	encoded, err := json.Marshal(reqBody)
	if err != nil {
		logger.Error("failed to marshal request body", err)
		return
	}

	req, err := http.NewRequest("POST", settings.Scrobbling().ListenBrainzUrl()+"/1/submit-listens", bytes.NewBuffer(encoded))
	if err != nil {
		logger.Error("failed to create request to listenbrainz", err)
		return
	}
	req.Header.Set("Authorization", "Token "+settings.Scrobbling().ListenBrainzToken())

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		logger.Error("failed to send request to listenbrainz", err)
		return
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		body, err := io.ReadAll(resp.Body)
		if err != nil {
			logger.Error("failed to read response body", err)
			return
		}
		logger.Error("listenbrainz returned non-200 status code", "status", resp.StatusCode, "body", string(body))
		return
	}
}

func (scrobbler *ListenBrainzScrobblerType) GetName() string {
	return "ListenBrainz"
}

func (scrobbler *ListenBrainzScrobblerType) IsConfigured() bool {
	if !settings.Scrobbling().ShouldEnableListenBrainz() {
		logger.Debug("skipping scrobbling to listenbrainz because it is disabled")
		return false
	}

	if settings.Scrobbling().ListenBrainzToken() == "" {
		logger.Debug("skipping scrobbling to listenbrainz because API key is not set")
		return false
	}
	return true
}

func (scrobbler *ListenBrainzScrobblerType) generateRequest(track tonearm.Track, listenType listenBrainzListenType) listenBrainzRequest {
	return listenBrainzRequest{
		ListenType: listenType,
		Payload: []listenBrainzPayload{
			{
				TrackMetadata: listenBrainzTrackMetadata{
					TrackName:  track.Title(),
					ArtistName: strings.Join(track.Artists().Names(), ", "),
					AdditionalInfo: listenBrainzAdditionalInfo{
						MediaPlayer:      "Tonearm",
						MusicService:     "tidal.com",
						OriginURL:        track.URL(),
						SubmissionClient: "Tonearm",
						DurationMs:       int(track.Duration().Milliseconds()),
					},
				},
			},
		},
	}
}

type listenBrainzRequest struct {
	ListenType listenBrainzListenType `json:"listen_type"`
	Payload    []listenBrainzPayload  `json:"payload"`
}

type listenBrainzPayload struct {
	ListenedAt    int64                     `json:"listened_at,omitempty"`
	TrackMetadata listenBrainzTrackMetadata `json:"track_metadata"`
}

type listenBrainzTrackMetadata struct {
	AdditionalInfo listenBrainzAdditionalInfo `json:"additional_info"`
	ArtistName     string                     `json:"artist_name"`
	TrackName      string                     `json:"track_name"`
}

type listenBrainzAdditionalInfo struct {
	MediaPlayer      string `json:"media_player"`
	MusicService     string `json:"music_service"`
	OriginURL        string `json:"origin_url"`
	SubmissionClient string `json:"submission_client"`
	DurationMs       int    `json:"duration_ms"`
	ISRC             string `json:"isrc"`
}
