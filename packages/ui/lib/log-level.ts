import { ILogLevel } from "./schema";

export function logLevelFromNumber(logLevel: number): ILogLevel {
	switch (logLevel) {
		case 0:
			return ILogLevel.Debug;
		case 1:
			return ILogLevel.Info;
		case 2:
			return ILogLevel.Warn;
		case 3:
			return ILogLevel.Error;
		case 4:
			return ILogLevel.Fatal;
		default:
			throw new Error(`Invalid log level: ${logLevel}`);
	}
}
