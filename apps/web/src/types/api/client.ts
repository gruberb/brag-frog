// Hand-written because ts-rs only exports types that derive TS; the error body is shared by every endpoint.
export type ApiErrorBody = { error: { code: string; message: string } };
