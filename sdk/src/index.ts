import enclzIdl from "./enclz.json";

export type { Enclz } from "./enclz";

export const IDL = enclzIdl;

export const PROGRAM_ID: string = IDL.address;
