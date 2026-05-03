import type { Enclz } from "./enclz";
import enclzIdl from "./enclz.json";

export type { Enclz };

export const IDL = enclzIdl as unknown as Enclz;

export const PROGRAM_ID: string = IDL.address;
