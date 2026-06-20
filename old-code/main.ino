// 6/14/26 - Changed the probes being checked


#include<Wire.h>
#include<LiquidCrystal_I2C.h>
#include "TCA9548.h"


//Relays, pins


int const FANLEFT_0 = 9;
int const FANRIGHT_1 = 10;
int const PUMPRELAY_2 = 8;


// true false to see if pumps/fans are on or not


bool relay_fan_0 = 0;
bool relay_fan_1 = 0;
bool relay_pump = 0;


// setting min time fans and pump to be on


const unsigned long MIN_ON_TIME_FAN = 3UL * 60UL * 1000UL; // 3 minutes (can change first number to change minutes)
const unsigned long MIN_ON_TIME_PUMP = 1UL * 60UL * 1000UL; // 1 minutes
const unsigned long MIN_LOCKOUT_TIME = 5UL * 60UL * 1000UL; // 5 minutes


// setting variables for fan left, fan right, and pump timer


unsigned long fanLeftTurnOnAt = 0;
unsigned long fanRightTurnOnAt = 0;
unsigned long PumpTurnOnAt = 0;
unsigned long leftFanLockout = 0;
unsigned long rightFanLockout = 0;
unsigned long pumpLockout = 0;


// LCD Displays


LiquidCrystal_I2C lcd1(0x27, 20, 4);
LiquidCrystal_I2C lcd2(0x26, 20, 4);


// Multiplexer object


TCA9548 mux(0x70);


// Read the Sensors


const uint8_t Sensor_ADDR = 0x44;


// Devices Connected to Multiplexer


const uint8_t Sensor_0 = 0;
const uint8_t Sensor_1 = 1;
const uint8_t Sensor_2 = 2;
const uint8_t Sensor_3 = 3;


// Temperature and Humidity Conversion (deprecated)


  // float temps_conversion = ((-45.0 + 175.0 * (temp_raw / 65535.0)) * (9.0 / 5.0)) + 32.0; // renamed to not confuse with name of array
  // float humis_conversion = 100.0 * (humi_raw / 65535.0); // renamed to not confuse with name of array


// declaring temp and humi array as global arrays to avoid issue


float temps[4];
float humis[4];


// Thresholds for Temperature/Humidity


const float TempThresh_0 = 75.0;
const float TempThresh_1 = 80.0; // not used
const float TempThresh_2 = 85.0; // Temp Higher than 85 is DANGEROUS
const float HumiThresh_0 = 70.0;
const float HumiThresh_1 = 65.0; // Lower than 65% is DANGEROUS


// Read Sensor Data // re witing whole ting


void ReadSensor(uint8_t channel){ //changed array declaration
  // selecting which thergrometer to check
  mux.selectChannel(channel);


  // Measurement command
    Wire.beginTransmission(Sensor_ADDR);
    Wire.write(0x24);
    Wire.write(0x00);
    int error = Wire.endTransmission();


  // Error catching
  if (error != 0){
    temps[channel] = -999.0; // changed to change value in array
    humis[channel] = -999.0; // changed to change value in array


  }
  else
  {
    delay(20);
    Wire.requestFrom(Sensor_ADDR, 6);
    if (Wire.available() == 6) {
    // Read temperature data
    uint16_t temps_raw = (Wire.read() << 8) | Wire.read();
    Wire.read();


    // Read humidity data
    uint16_t humis_raw = (Wire.read() << 8) | Wire.read();
    Wire.read();


    // In future, want to not hard code which temp its saving to, unsure best way to do yet. For now hard coded
    temps[channel] = (((-45.0 + 175.0 * (temps_raw / 65535.0)) * (9.0 / 5.0)) + 32.0);
    humis[channel] = (100.0 * (humis_raw / 65535.0));
    }
    else // there is an error
    {
      temps[channel] = -999.0; // changed to change value in array
      humis[channel] = -999.0; // changed to change value in array


    }
  }
  return;
}


// LCD Custom Characters


byte PumpIcon[] = { // 0
  B00000,
  B01110,
  B11111,
  B11111,
  B01010,
  B01010,
  B01010,
  B01110
};
byte PumpSprayL[] = { // 1
  B00100,
  B00010,
  B01011,
  B00011,
  B00110,
  B01010,
  B00100,
  B00000
};
byte PumpSprayR[] = { // 2
  B00100,
  B01000,
  B11010,
  B11000,
  B01100,
  B01010,
  B00100,
  B00000
};
byte Fly[] = { // 4
  B00000,
  B00000,
  B00000,
  B01100,
  B01101,
  B01110,
  B01110,
  B01010
};
byte Frog[] = { // 3
  B00000,
  B00011,
  B00111,
  B01111,
  B01111,
  B11111,
  B10001,
  B11101
};


// LCD Display Control


void UpdateDisplays() {


// LCD1 Display


lcd1.setCursor(0, 0);
lcd1.print("Kin'iro & Gin'iro");
lcd1.setCursor(0, 1);
lcd1.print("Temp and Humidity:");
lcd1.setCursor(0, 2);
lcd1.createChar(0, PumpIcon);
lcd1.setCursor(18, 3);
lcd1.write(0);
lcd1.createChar(3, Frog);
lcd1.setCursor(15, 2);
lcd1.write(3);
lcd1.createChar(3, Frog);
lcd1.setCursor(14, 2);
lcd1.write(3);
lcd1.createChar(4, Fly);
lcd1.setCursor(17, 2);
lcd1.write(4);


if (temps[0] == -999.0) {
    lcd1.setCursor(0, 2);
    lcd1.print("ERROR");
   } else{
    lcd1.setCursor(0, 2);
    lcd1.print(temps[0], 2);
    lcd1.print("F");
    lcd1.setCursor(0, 3);
    lcd1.print(humis[0], 2);
    lcd1.print("%");
  }
if (temps[1] == -999.0) {
    lcd1.setCursor(6, 2);
    lcd1.print("ERROR");
   } else{
    lcd1.setCursor(6, 2);
    lcd1.print("|");
    lcd1.setCursor(7, 2);
    lcd1.print(temps[1], 2);
    lcd1.print("F");
    lcd1.setCursor(6, 3);
    lcd1.print("|");
    lcd1.setCursor(7, 3);
    lcd1.print(humis[1], 2);
    lcd1.print("%");
  }
  if (relay_pump == 1) {
    lcd1.createChar(1, PumpSprayL);
    lcd1.setCursor(17, 3);
    lcd1.write(1);
    lcd1.createChar(2, PumpSprayR);
    lcd1.setCursor(19, 3);
    lcd1.write(2);
    delay(1000);
    lcd1.setCursor(17, 3);
    lcd1.print(" ");
    lcd1.setCursor(19, 3);
    lcd1.print(" ");
    delay(1000);
    lcd1.createChar(1, PumpSprayL);
    lcd1.setCursor(17, 3);
    lcd1.write(1);
    lcd1.createChar(2, PumpSprayR);
    lcd1.setCursor(19, 3);
    lcd1.write(2);
    delay(1000);
    lcd1.setCursor(17, 3);
    lcd1.print(" ");
    lcd1.setCursor(19, 3);
    lcd1.print(" ");
  }


// Lcd2 Display


lcd2.setCursor(0, 0);
lcd2.print("Marble,Granite,Onyx");
lcd2.setCursor(0, 1);
lcd2.print("Temp and Humidity:");
lcd2.setCursor(0, 2);
lcd2.createChar(0, PumpIcon);
lcd2.setCursor(18, 3);
lcd2.write(0);
lcd2.createChar(3, Frog);
lcd2.setCursor(15, 2);
lcd2.write(3);
lcd2.createChar(3, Frog);
lcd2.setCursor(14, 2);
lcd2.write(3);
lcd2.createChar(3, Frog);
lcd2.setCursor(13, 2);
lcd2.write(3);
lcd2.createChar(4, Fly);
lcd2.setCursor(17, 2);
lcd2.write(4);


if (temps[2] == -999.0) {
    lcd2.setCursor(0, 2);
    lcd2.print("ERROR");
   } else{
    lcd2.setCursor(0, 2);
    lcd2.print(temps[2], 2);
    lcd2.print("F");
    lcd2.setCursor(0, 3);
    lcd2.print(humis[2], 2);
    lcd2.print("%");
  }
if (temps[3] == -999.0) {
    lcd2.setCursor(0, 3);
    lcd2.print("ERROR");
   } else{
    lcd2.setCursor(6, 2);
    lcd2.print("|");
    lcd2.setCursor(7, 2);
    lcd2.print(temps[3], 2);
    lcd2.print("F");
    lcd2.setCursor(6, 3);
    lcd2.print("|");
    lcd2.setCursor(7, 3);
    lcd2.print(humis[3], 2);
    lcd2.print("%");
  }
  if (relay_pump == 1) {
    lcd2.createChar(1, PumpSprayL);
    lcd2.setCursor(17, 3);
    lcd2.write(1);
    lcd2.createChar(2, PumpSprayR);
    lcd2.setCursor(19, 3);
    lcd2.write(2);
    delay(1000);
    lcd2.setCursor(17, 3);
    lcd2.print(" ");
    lcd2.setCursor(19, 3);
    lcd2.print(" ");
    delay(1000);
    lcd2.createChar(1, PumpSprayL);
    lcd2.setCursor(17, 3);
    lcd2.write(1);
    lcd2.createChar(2, PumpSprayR);
    lcd2.setCursor(19, 3);
    lcd2.write(2);
    delay(1000);
    lcd2.setCursor(17, 3);
    lcd2.print(" ");
    lcd2.setCursor(19, 3);
    lcd2.print(" ");
  }
}


// Relay Controls


void ControlRelays() {
unsigned long now = millis(); // getting start time
  // possible changes in the future
    // int templeft = temp[1];
    // bool istempover85 = temps[1] > TempThresh_2;




// Error checking on sensors (change hard coded 5 and -999 at somepoint)
 
  bool leftHumiValid  = humis[1] > 10 && humis[1] != -999;
  bool rightHumiValid = humis[2] > 10 && humis[2] != -999;


  bool leftHumiError  = humis[1] == -999;
  bool rightHumiError = humis[2] == -999;


  bool anyHumiError = leftHumiError || rightHumiError;


// Temperature check


  bool leftTempAboveFanThresh  = temps[1] > TempThresh_0;
  bool rightTempAboveFanThresh = temps[2] > TempThresh_0;


  bool leftTempDanger  = temps[1] > TempThresh_2;
  bool rightTempDanger = temps[2] > TempThresh_2;


  bool leftTempGoodForFanOff  = temps[1] <= TempThresh_0;
  bool rightTempGoodForFanOff = temps[2] <= TempThresh_0;


  bool leftTempGoodForPumpOff  = temps[1] <= TempThresh_2;
  bool rightTempGoodForPumpOff = temps[2] <= TempThresh_2;


// Humidity checks


  bool leftHumidityLow  = humis[1] < HumiThresh_0 && leftHumiValid;
  bool rightHumidityLow = humis[2] < HumiThresh_0 && rightHumiValid;


  bool leftHumidityGood  = humis[1] >= HumiThresh_0 && leftHumiValid;
  bool rightHumidityGood = humis[2] >= HumiThresh_0 && rightHumiValid;


// Relay state check
 
  bool leftFanIsOff  = relay_fan_0 == false;
  bool rightFanIsOff = relay_fan_1 == false;
  bool pumpIsOff     = relay_pump == false;


  bool leftFanIsOn  = relay_fan_0 == true;
  bool rightFanIsOn = relay_fan_1 == true;
  bool pumpIsOn     = relay_pump == true;


// Min Run Time Check
  bool leftFanRanLongEnough =
    now - fanLeftTurnOnAt >= MIN_ON_TIME_FAN;


  bool rightFanRanLongEnough =
    now - fanRightTurnOnAt >= MIN_ON_TIME_FAN;


  bool pumpRanLongEnough =
    now - PumpTurnOnAt >= MIN_ON_TIME_PUMP;
 
// Add fanPumpLockout at top, aswell as min lockout time
  bool leftFanLockoutDone =
    now - leftFanLockout >= MIN_LOCKOUT_TIME;
  bool rightFanLockoutDone =
    now - rightFanLockout >= MIN_LOCKOUT_TIME;
  bool pumpLockoutDone =
    now - pumpLockout >= MIN_LOCKOUT_TIME;


// Combined checks
  // Fan should turn on if temp is above normal fan threshold and lockout is done
  bool shouldTurnOnLeftFan =
    leftTempAboveFanThresh && leftFanIsOff && leftFanLockoutDone;


  bool shouldTurnOnRightFan =
    rightTempAboveFanThresh && rightFanIsOff && rightFanLockoutDone;


  // Pump should turn on if either vivarium is dangerously hot
  // or either humidity is too low
  bool shouldTurnOnPump =
    pumpIsOff &&
    pumpLockoutDone &&
    (
      leftTempDanger ||
      rightTempDanger ||
      leftHumidityLow ||
      rightHumidityLow
    );


  // Fans turn off only when temp is good again and min on-time passed
  bool shouldTurnOffLeftFan =
    leftFanIsOn &&
    leftTempGoodForFanOff &&
    leftFanRanLongEnough;


  bool shouldTurnOffRightFan =
    rightFanIsOn &&
    rightTempGoodForFanOff &&
    rightFanRanLongEnough;


  // Pump turns off if:
  // 1. pump is on
  // 2. both vivariums are safe enough
  // 3. both humidity readings are good
  // 4. pump has run long enough
  bool bothVivariumGoodForPump =
    leftTempGoodForPumpOff &&
    rightTempGoodForPumpOff &&
    leftHumidityGood &&
    rightHumidityGood;


  bool shouldTurnOffPumpNormally =
    pumpIsOn &&
    bothVivariumGoodForPump &&
    pumpRanLongEnough;


  // Optional emergency/error pump shutoff
  bool shouldTurnOffPumpBecauseError =
    pumpIsOn && anyHumiError;


  bool shouldTurnOffPump =
    shouldTurnOffPumpNormally || shouldTurnOffPumpBecauseError;


// Relay Controls


if (shouldTurnOnLeftFan) {
    relay_fan_0 = true;
    digitalWrite(FANLEFT_0, HIGH);
    fanLeftTurnOnAt = now;
  }


  if (shouldTurnOnRightFan) {
    relay_fan_1 = true;
    digitalWrite(FANRIGHT_1, HIGH);
    fanRightTurnOnAt = now;
  }


  if (shouldTurnOnPump) {
    relay_pump = true;
    digitalWrite(PUMPRELAY_2, HIGH);
    PumpTurnOnAt = now;
  }


  if (shouldTurnOffLeftFan) {
    relay_fan_0 = false;
    digitalWrite(FANLEFT_0, LOW);
    leftFanLockout = now;
  }


  if (shouldTurnOffRightFan) {
    relay_fan_1 = false;
    digitalWrite(FANRIGHT_1, LOW);
    rightFanLockout = now;
  }


  if (shouldTurnOffPump) {
    relay_pump = false;
    digitalWrite(PUMPRELAY_2, LOW);
    pumpLockout = now;
  }
}


void setup() {
  // put your setup code here, to run once:


Serial.begin(9600);


// setting lockout so can run fans and pump at start
leftFanLockout = millis() - MIN_LOCKOUT_TIME;
rightFanLockout = millis() - MIN_LOCKOUT_TIME;
pumpLockout = millis() - MIN_LOCKOUT_TIME;


// Setup Relays


pinMode(FANLEFT_0, OUTPUT);
digitalWrite(FANLEFT_0, LOW);
pinMode(FANRIGHT_1, OUTPUT);
digitalWrite(FANRIGHT_1, LOW);
pinMode(PUMPRELAY_2, OUTPUT);
digitalWrite(PUMPRELAY_2, LOW);


// setting lockout so can run fans and pump at start
leftFanLockout = millis() - MIN_LOCKOUT_TIME;
rightFanLockout = millis() - MIN_LOCKOUT_TIME;
pumpLockout = millis() - MIN_LOCKOUT_TIME;


// Start Multiplexer  


Wire.begin();       // Start Arduino I2C
mux.begin();        // Start/Configure Multiplexer
delay(100);


 // Start LCD Screens


  lcd1.init();
  lcd1.backlight();
  lcd1.setCursor(0, 0);
  lcd1.print("Please Wait");
  lcd2.init();
  lcd2.backlight();
  lcd2.setCursor(0, 0);
  lcd2.print("Loading...");


}


void loop() {
  // put your main code here, to run repeatedly:


// Sensor


for (uint8_t i = 0; i < 4; i++)
{
  ReadSensor(i); // only need to send current i value
}


// Getta Da Time


//However that goes r/GetInternet


// Weelays


ControlRelays();


// El SeeDeez


UpdateDisplays();


delay(5000);
}



