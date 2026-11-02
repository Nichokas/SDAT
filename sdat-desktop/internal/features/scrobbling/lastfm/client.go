package lastfm

import (
	"context"
	"log/slog"
	"time"

	"codeberg.org/dergs/tonearm/internal/settings"
	"github.com/twoscott/gobble-fm/api"
	"github.com/twoscott/gobble-fm/session"
)

var logger = slog.With("module", "scrobbler/lastfm")

type Client struct {
	*session.Client
}

const (
	apiKey = "9691195478432f52cee57cc2a1b8b066"
	secret = "3544f7702f11faea922c586be7b4df63"
)

func NewClient() *Client {
	key := settings.Scrobbling().LastFMToken()
	client := session.NewClient(apiKey, secret)
	if key != "" {
		client.SetSessionKey(key)
	}

	return &Client{client}
}

func (client *Client) StartPollingSession(ctx context.Context, token string) (ok bool, err error) {
	for {
		select {
		case <-ctx.Done():
			logger.Info("Last.fm configuration cancelled by user", "message", ctx.Err())
			return false, nil
		case <-time.After(5 * time.Second):
			logger.Debug("Polling for Last.fm session")

			err := client.TokenLogin(token)
			if err == nil {
				return true, nil
			} else if err, ok := err.(*api.LastFMError); ok {
				switch err.Code {
				case api.ErrUnauthorizedToken:
					continue
				default:
					return false, err
				}
			} else {
				return false, err
			}
		}
	}
}
