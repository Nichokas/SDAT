package state

import (
	"context"
	"log/slog"
	"sync"
	"time"

	"codeberg.org/dergs/tonearm/internal/secrets"
	"codeberg.org/dergs/tonearm/pkg/tidalapi"
	v1 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v1"
	v2 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v2"
	"github.com/infinytum/injector"
)

type FavouriteCacheV2 struct {
	etag        string
	items       *[]string
	lastFetched time.Time
	mutex       sync.RWMutex
	resource    interface {
		Add(context.Context, string) error
		Remove(context.Context, string) error
		IDs(context.Context, string, string) (*v2.FavouritesIds, string, error)
	}
}

type FavouriteCacheV1 struct {
	etag        string
	items       *[]string
	lastFetched time.Time
	mutex       sync.RWMutex
	listGetter  func(*v1.FavouritesIdLists) []string
	resource    interface {
		Add(context.Context, string, string) error
		Remove(context.Context, string, string) error
	}
}

type FavouriteCache interface {
	Add(string) error
	Bust()
	Get() (*[]string, error)
	Remove(string) error
}

var (
	AlbumsCache    = &FavouriteCacheV1{}
	ArtistsCache   = &FavouriteCacheV1{}
	MixesCache     = &FavouriteCacheV2{}
	PlaylistsCache = &FavouriteCacheV1{}
	TracksCache    = &FavouriteCacheV1{}
)

var wasInitialized bool

func deferredInit() {
	if wasInitialized {
		return
	}
	wasInitialized = true

	tidal, _ := injector.Inject[*tidalapi.TidalAPI]()

	AlbumsCache.resource = tidal.V1.Favourites.Albums
	AlbumsCache.listGetter = func(lists *v1.FavouritesIdLists) []string {
		return lists.Album
	}

	ArtistsCache.resource = tidal.V1.Favourites.Artists
	ArtistsCache.listGetter = func(lists *v1.FavouritesIdLists) []string {
		return lists.Artist
	}

	MixesCache.resource = tidal.V2.Favourites.Mixes

	PlaylistsCache.resource = tidal.V1.Favourites.Playlists
	PlaylistsCache.listGetter = func(lists *v1.FavouritesIdLists) []string {
		return lists.Playlist
	}

	TracksCache.resource = tidal.V1.Favourites.Tracks
	TracksCache.listGetter = func(lists *v1.FavouritesIdLists) []string {
		return lists.Track
	}
}

func (cache *FavouriteCacheV1) Add(id string) error {
	userId := secrets.UserID()
	if userId == "" {
		slog.Debug("not logged in, ignoring favourite addition")
		return nil
	}

	deferredInit()
	cache.mutex.Lock()
	defer func() {
		cache.mutex.Unlock()
		cache.Bust()
	}()

	return cache.resource.Add(context.Background(), userId, id)
}

func (cache *FavouriteCacheV1) Get() (*[]string, error) {
	userId := secrets.UserID()
	if userId == "" {
		slog.Debug("not logged in, ignoring favourite fetch")
		return &[]string{}, nil
	}

	deferredInit()
	cache.mutex.RLock()
	if cache.items != nil && cache.lastFetched.Add(5*time.Minute).After(time.Now()) {
		defer cache.mutex.RUnlock()
		return cache.items, nil
	}

	cache.mutex.RUnlock()

	cache.mutex.Lock()
	defer cache.mutex.Unlock()

	tidal, _ := injector.Inject[*tidalapi.TidalAPI]()
	lists, etag, notModified, err := tidal.V1.Favourites.IDsWithCache(context.Background(), userId, cache.etag)
	if err != nil {
		return nil, err
	}

	cache.lastFetched = time.Now()

	if notModified {
		return cache.items, nil
	}

	list := cache.listGetter(lists)
	cache.items = &list
	cache.etag = etag

	return cache.items, nil
}

func (cache *FavouriteCacheV1) Bust() {
	cache.mutex.Lock()
	defer cache.mutex.Unlock()

	cache.lastFetched = time.Time{}
}

func (cache *FavouriteCacheV1) Remove(id string) error {
	userId := secrets.UserID()
	if userId == "" {
		slog.Debug("not logged in, ignoring favourite removal")
		return nil
	}

	deferredInit()
	cache.mutex.Lock()
	defer func() {
		cache.mutex.Unlock()
		cache.Bust()
	}()

	return cache.resource.Remove(context.Background(), userId, id)
}

func (cache *FavouriteCacheV2) Add(id string) error {
	userId := secrets.UserID()
	if userId == "" {
		slog.Debug("not logged in, ignoring favourite addition")
		return nil
	}

	deferredInit()
	cache.mutex.Lock()
	defer func() {
		cache.mutex.Unlock()
		cache.Bust()
	}()

	return cache.resource.Add(context.Background(), id)
}

func (cache *FavouriteCacheV2) Remove(id string) error {
	userId := secrets.UserID()
	if userId == "" {
		slog.Debug("not logged in, ignoring favourite removal")
		return nil
	}

	deferredInit()
	cache.mutex.Lock()
	defer func() {
		cache.mutex.Unlock()
		cache.Bust()
	}()

	return cache.resource.Remove(context.Background(), id)
}

func (cache *FavouriteCacheV2) Get() (*[]string, error) {
	userId := secrets.UserID()
	if userId == "" {
		slog.Debug("not logged in, ignoring favourite fetch")
		return &[]string{}, nil
	}

	deferredInit()
	cache.mutex.RLock()
	if cache.items != nil && cache.lastFetched.Add(5*time.Minute).After(time.Now()) {
		defer cache.mutex.RUnlock()
		return cache.items, nil
	}

	cache.mutex.RUnlock()

	cache.mutex.Lock()
	defer cache.mutex.Unlock()

	list, etag, err := cache.resource.IDs(context.Background(), "", cache.etag)
	if err != nil {
		return nil, err
	}

	cache.lastFetched = time.Now()

	if list == nil {
		return cache.items, nil
	}

	cache.items = &list.Content

	for list.Cursor != "" {
		list, etag, err = cache.resource.IDs(context.Background(), list.Cursor, etag)
		if err != nil {
			return nil, err
		}

		*cache.items = append(*cache.items, list.Content...)
	}

	cache.etag = etag

	return cache.items, nil
}

func (cache *FavouriteCacheV2) Bust() {
	cache.mutex.Lock()
	defer cache.mutex.Unlock()

	cache.lastFetched = time.Time{}
}
