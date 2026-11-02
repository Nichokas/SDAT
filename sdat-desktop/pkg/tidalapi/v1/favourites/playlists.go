package favourites

import (
	"context"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"

	"codeberg.org/dergs/tonearm/pkg/tidalapi/internal"
)

type FavouritePlaylist struct {
	client *internal.Client
}

func NewFavouritePlaylist(client *internal.Client) *FavouritePlaylist {
	return &FavouritePlaylist{
		client: client,
	}
}

func (f *FavouritePlaylist) Add(ctx context.Context, userID, playlistUUID string) error {
	body := url.Values{
		"uuids":              {playlistUUID},
		"onArtifactNotFound": {"FAIL"},
	}
	req, err := http.NewRequestWithContext(
		ctx,
		http.MethodPost,
		fmt.Sprintf("/v1/users/%s/favorites/playlists", userID),
		strings.NewReader(body.Encode()),
	)
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	if err != nil {
		return err
	}

	resp, err := f.client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		errorText, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("error while adding playlist to favourites: %s", errorText)
	}

	return nil
}

func (f *FavouritePlaylist) Remove(ctx context.Context, userID, playlistUUID string) error {
	req, err := http.NewRequestWithContext(ctx, http.MethodDelete, fmt.Sprintf("/v1/users/%s/favorites/playlists/%s", userID, playlistUUID), nil)
	if err != nil {
		return err
	}

	resp, err := f.client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return err
	}

	return nil
}
