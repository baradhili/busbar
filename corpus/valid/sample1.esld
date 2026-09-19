// Sample 1 — grid-only house with garage sub-board.
// Converted from the RSLD seed (Design/deepseek.md §16.1) to ESLD v0.1
// per Design/esld-spec.md Appendix A.

profile "esld/1.0";

voltsys LV = 230V, 1ph, 50Hz;

code "AS/NZS 3000:2018" {
  rcd_socket_circuits = 30mA;
  max_final_circuit_a = 32A;
  spd_required        = yes;
};

grid GRID : grid { vs = LV; supply_a = 63A; fault_mva = 1.38MVA; };  // 6kA at 230V

board MAIN : board {
  vs              = LV;
  incomers        = [GRID.out];
  busbar_rating_a = 100A;
  location        = "Garage";
  ways            = 24;

  main_switch MSB : main_switch {
    rating_a = 63A;
    poles    = 2;
  };

  spd SPD1 : spd { spd_type = 2; up_kv = 1.5kV; in_ka = 20kA; };

  circuit LIGHTS {
    protection : rcbo { rating_a = 10A; curve = B; rcd_ma = 30mA; };
    cable      = { csa = 1.5mm2; cores = 3; method = "clipped"; };
    loads      = [LIGHT_HALL, LIGHT_LOUNGE, LIGHT_KITCHEN];
  };

  circuit SOCKETS_GF {
    protection : rcbo { rating_a = 20A; curve = C; rcd_ma = 30mA; };
    cable      = { csa = 2.5mm2; cores = 3; method = "clipped"; };
    loads      = [SOCK_KITCHEN, SOCK_LOUNGE, SOCK_DINING];
  };

  circuit OVEN {
    protection : mcb { rating_a = 32A; curve = C; poles = 2; };
    cable      = { csa = 6mm2; cores = 3; };
    loads      = [OVEN1];
  };

  circuit GARAGE_FEED {
    protection : rcbo { rating_a = 40A; curve = C; rcd_ma = 30mA; };
    cable      = { csa = 6mm2; cores = 3; length = 18m; };
  };
};

board GARAGE : board {
  vs              = LV;
  incomers        = [MAIN.GARAGE_FEED.out];
  busbar_rating_a = 63A;
  ways            = 12;

  circuit GARAGE_LIGHTS {
    protection : rcbo { rating_a = 10A; curve = B; rcd_ma = 30mA; };
    loads      = [LIGHT_GARAGE];
  };

  circuit GARAGE_SOCKETS {
    protection : rcbo { rating_a = 20A; curve = C; rcd_ma = 30mA; };
    loads      = [SOCK_GARAGE, SOCK_WORKBENCH];
  };
};

connect GRID.out       -> MAIN.MSB.in;
connect MAIN.MSB.out   -> MAIN.bus;
connect MAIN.GARAGE_FEED.out -> GARAGE.bus;

// Loads
lighting LIGHT_HALL     : lighting { kw = 0.06kW; points = 1; };
lighting LIGHT_LOUNGE   : lighting { kw = 0.12kW; points = 2; };
lighting LIGHT_KITCHEN  : lighting { kw = 0.12kW; points = 2; };
socket  SOCK_KITCHEN    : socket  { kw = 2.4kW; points = 4; };
socket  SOCK_LOUNGE     : socket  { kw = 1.6kW; points = 3; };
socket  SOCK_DINING     : socket  { kw = 1.6kW; points = 2; };
oven    OVEN1           : oven    { kw = 7.2kW; };
lighting LIGHT_GARAGE   : lighting { kw = 0.06kW; points = 1; };
socket  SOCK_GARAGE     : socket  { kw = 1.6kW; points = 2; };
socket  SOCK_WORKBENCH  : socket  { kw = 2.4kW; points = 2; };