package lyrics

import (
	"context"
	"regexp"
	"strconv"
	"strings"
	"time"

	"codeberg.org/dergs/tonearm/internal/player"
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/dergs/tonearm/pkg/tidalapi"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/models/openapi"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"github.com/infinytum/injector"
)

func getLyrics(ID string) (lyrics string, isTimestamped bool, err error) {
	tidal := injector.MustInject[*tidalapi.TidalAPI]()
	var track *openapi.Track
	track, err = tidal.OpenAPI.V2.Tracks.Track(context.Background(), ID, "lyrics")
	if err != nil {
		return
	}

	for _, lyric := range track.Included.PlainLyrics(track.Data.Relationships.Lyrics.Data...) {
		if lyric.Attributes.LRCText != "" {
			isTimestamped = true
			lyrics = lyric.Attributes.LRCText
		} else if lyric.Attributes.Text != "" {
			isTimestamped = false
			lyrics = lyric.Attributes.Text
		}
		break
	}

	return
}

func parseLRCLyrics(lyrics string, trackDuration time.Duration) (lines []any) {
	// Handle lyrics with timings
	// Remove timing tags and split into lines
	timingRegex := regexp.MustCompile(`\[(\d{2}:\d{2}\.\d{2})\](.*)`)
	splitLyrics := strings.Split(lyrics, "\n")
	timings := []*highlightTiming{}

	for i, line := range splitLyrics {
		// Skip empty lines
		if strings.TrimSpace(line) == "" {
			lines = append(lines, Box(gtk.OrientationVerticalValue))
			continue
		}

		matches := timingRegex.FindStringSubmatch(line)
		if len(matches) < 3 {
			continue
		}

		timestampStart := matches[1]
		timeStart, _ := parseTimestamp(timestampStart)

		var timeEnd time.Duration = trackDuration

		if i+1 < len(splitLyrics) {
			offset := 1
			nextLineMatches := timingRegex.FindStringSubmatch(splitLyrics[i+offset])

			for len(nextLineMatches) < 2 && i+offset+1 < len(splitLyrics) {
				offset++
				nextLineMatches = timingRegex.FindStringSubmatch(splitLyrics[i+offset])
			}

			if len(nextLineMatches) >= 2 {
				timestampEnd := nextLineMatches[1]
				timeEnd, _ = parseTimestamp(timestampEnd)
			}
		}

		if matches[2] == "" {
			timings = append(timings, &highlightTiming{
				Start: timeStart,
				End:   timeEnd,
			})

			continue
		}

		lyricText := line
		if len(matches) >= 3 {
			// Extract just the lyric text, removing the timing
			lyricText = strings.TrimSpace(matches[2])
		}

		timing := &highlightTiming{
			Start: timeStart,
			End:   timeEnd,
		}
		boxWidget := lyricLine(
			lyricText,
			timing,
		)()

		lines = append(lines, boxWidget)

		timing.Ref = weak.NewWidgetRef(boxWidget)

		timings = append(timings, timing)
	}

	activeIndexChangeOnPlayerUpdate = player.PlaybackStateChanged.On(func(state player.PlaybackState) (next bool) {
		next = signals.Continue
		if state.Status != player.PlaybackStatusPlaying {
			return
		}

		hasActive := false

		for _, timing := range timings {
			if state.Position > timing.End {
				continue
			}

			if timing.Start <= state.Position && timing.Ref != nil {
				timing.Ref.Use(func(obj *gtk.Widget) {
					if activeLyricIndex.Value() != obj.Ptr {
						setNewIndex(timing)
					}
				})

				hasActive = true
				continue
			}

			if timing.Start <= state.Position+(250*time.Millisecond) && timing.Ref != nil {
				logger.Debug("next lyric line scheduled", "timing", timing.Start-state.Position)
				time.AfterFunc(timing.Start-state.Position, func() {
					timing.Ref.Use(func(obj *gtk.Widget) {
						if activeLyricIndex.Value() != obj.Ptr {
							setNewIndex(timing)
						}
					})
				})

				continue
			}
		}

		if !hasActive && activeLyricIndex.Value() != 0 {
			schwifty.OnMainThreadOncePure(func() {
				activeLyricIndex.SetValue(0)
			})
		}

		return
	})

	return
}

func parseUntimedLyrics(lyrics string) (lines []any) {
	// Handle lyrics without timings
	splitLyrics := strings.SplitSeq(lyrics, "\n")

	for lyricText := range splitLyrics {
		if strings.TrimSpace(lyricText) == "" {
			lines = append(lines, Box(gtk.OrientationVerticalValue))
			continue
		}

		boxWidget := lyricLine(lyricText, nil)()
		lines = append(lines, boxWidget)
	}

	return
}

func parseTimestamp(timestamp string) (resTime time.Duration, ok bool) {
	timestampRegex := regexp.MustCompile(`(\d{2}):(\d{2})\.(\d{2})`)
	parts := timestampRegex.FindStringSubmatch(timestamp)

	if len(parts) != 4 {
		ok = false
		return
	}

	ok = true

	minutes, _ := strconv.Atoi(parts[1])
	seconds, _ := strconv.Atoi(parts[2])
	centiseconds, _ := strconv.Atoi(parts[3])

	resTime = time.Duration(minutes)*time.Minute + time.Duration(seconds)*time.Second + time.Duration(centiseconds)*time.Millisecond*10

	return
}
