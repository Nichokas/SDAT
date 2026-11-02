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

type FavouriteAlbum struct {
	client *internal.Client
}

func NewFavouriteAlbum(client *internal.Client) *FavouriteAlbum {
	return &FavouriteAlbum{
		client: client,
	}
}

func (f *FavouriteAlbum) Add(ctx context.Context, userID, albumId string) error {
	body := url.Values{
		"albumIds":           {albumId},
		"onArtifactNotFound": {"FAIL"},
	}
	req, err := http.NewRequestWithContext(
		ctx,
		http.MethodPost,
		fmt.Sprintf("/v1/users/%s/favorites/albums", userID),
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
		return fmt.Errorf("error while adding album to favourites: %s", errorText)
	}

	return nil
}

func (f *FavouriteAlbum) Remove(ctx context.Context, userID, albumId string) error {
	req, err := http.NewRequestWithContext(ctx, http.MethodDelete, fmt.Sprintf("/v1/users/%s/favorites/albums/%s", userID, albumId), nil)
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
