export interface IProfile {
    bits:        Array<string[]>;
    created:     string;
    description: string;
    hub?:        string;
    hubs?:       string[];
    id?:         string;
    name:        string;
    thumbnail:   string;
    updated:     string;
    [property: string]: any;
}
