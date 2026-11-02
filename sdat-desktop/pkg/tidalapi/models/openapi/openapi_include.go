package openapi

import (
	"encoding/json"

	"github.com/qjebbs/go-jsons"
)

type ObjectType string

type IncludedObjects []IncludedObject

func (i *IncludedObjects) UnmarshalJSON(b []byte) error {
	var temp []IncludedObject
	if err := json.Unmarshal(b, &temp); err != nil {
		return err
	}

	merged := make(map[string]IncludedObject)
	for _, obj := range temp {
		key := obj.Type + "||" + obj.ID
		if _, exists := merged[key]; !exists {
			merged[key] = obj
		} else {
			existing := merged[key]
			mergedRaw, err := jsons.Merge([]byte(existing.Raw), []byte(obj.Raw))
			if err != nil {
				return err
			}
			existing.Raw = json.RawMessage(mergedRaw)
			merged[key] = existing
		}
	}

	*i = make(IncludedObjects, 0)
	for _, obj := range merged {
		*i = append(*i, obj)
	}

	return nil
}

type IncludedObject struct {
	ID   string `json:"id"`
	Type string `json:"type"`
	Raw  json.RawMessage
}

func (i *IncludedObject) UnmarshalJSON(b []byte) error {
	i.Raw = append([]byte(nil), b...)
	type Alias IncludedObject
	return json.Unmarshal(b, (*Alias)(i))
}

func (i IncludedObjects) FromRelationships(relationships []Relationship, t string) IncludedObjects {
	included := make(IncludedObjects, 0)
	for _, rel := range relationships {
		for _, obj := range i {
			if obj.ID == rel.ID && (obj.Type == t || t == "") {
				included = append(included, obj)
			}
		}
	}
	return included
}

func (i IncludedObjects) FromType(t string) IncludedObjects {
	var included IncludedObjects
	for _, obj := range i {
		if obj.Type == t {
			included = append(included, obj)
		}
	}
	return included
}
