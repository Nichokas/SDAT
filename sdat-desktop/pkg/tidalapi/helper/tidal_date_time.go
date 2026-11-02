package helper

import (
	"encoding/json"
	"time"
)

type TidalDateTime struct {
	time.Time
}

func (r *TidalDateTime) MarshalJSON() ([]byte, error) {
	return json.Marshal(r.Time.Format("2006-01-02T15:04:05.000-0700"))
}

func (d *TidalDateTime) UnmarshalJSON(b []byte) error {
	var s string
	err := json.Unmarshal(b, &s)
	if err != nil {
		return err
	}

	t, err := time.Parse("2006-01-02T15:04:05.000-0700", s)
	if err != nil {
		return err
	}

	d.Time = t
	return nil
}
