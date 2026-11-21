#include <string.h>
#include <arpa/inet.h>
#include <sys/socket.h>

#include "esp_event.h"
#include "esp_log.h"
#include "esp_mac.h"
#include "esp_netif.h"
#include "esp_wifi.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "nvs_flash.h"

#define SSID "TENGU-HUNT"
#define PASSWORD "kurama-dojo-mpp3esnn"
#define CHANNEL 6
#define MAX_CLIENTS 10
#define MESSAGE "next code: sojobo-fcq9dc- and 14 more chars. they are on me."
#define PORT 31337
#define SEND_EVERY_MS 3000
// ponytail: tx power in 0.25 dBm steps (8 = 2 dBm, 80 = 20 dBm). Tune on the con floor so the dBm hunt has a slope.
#define TX_POWER_QUARTER_DBM 34

static const char *TAG = "tengu-hunt";

static void on_wifi_event(void *arg, esp_event_base_t base, int32_t id, void *data)
{
    if (id == WIFI_EVENT_AP_STACONNECTED) {
        wifi_event_ap_staconnected_t *e = data;
        ESP_LOGI(TAG, "join " MACSTR, MAC2STR(e->mac));
    } else if (id == WIFI_EVENT_AP_STADISCONNECTED) {
        wifi_event_ap_stadisconnected_t *e = data;
        ESP_LOGI(TAG, "leave " MACSTR, MAC2STR(e->mac));
    }
}

static esp_netif_t *start_ap(void)
{
    ESP_ERROR_CHECK(esp_netif_init());
    ESP_ERROR_CHECK(esp_event_loop_create_default());
    esp_netif_t *netif = esp_netif_create_default_wifi_ap();

    wifi_init_config_t init = WIFI_INIT_CONFIG_DEFAULT();
    ESP_ERROR_CHECK(esp_wifi_init(&init));
    ESP_ERROR_CHECK(esp_event_handler_register(WIFI_EVENT, ESP_EVENT_ANY_ID, on_wifi_event, NULL));

    wifi_config_t config = {
        .ap = {
            .ssid = SSID,
            .ssid_len = strlen(SSID),
            .password = PASSWORD,
            .channel = CHANNEL,
            .max_connection = MAX_CLIENTS,
            .authmode = WIFI_AUTH_WPA2_PSK,
            .ssid_hidden = 1,
            .pmf_cfg = { .required = false },
        },
    };
    ESP_ERROR_CHECK(esp_wifi_set_mode(WIFI_MODE_AP));
    ESP_ERROR_CHECK(esp_wifi_set_config(WIFI_IF_AP, &config));
    ESP_ERROR_CHECK(esp_wifi_start());
    ESP_ERROR_CHECK(esp_wifi_set_max_tx_power(TX_POWER_QUARTER_DBM));
    return netif;
}

static int open_broadcast_socket(void)
{
    int sock = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
    int on = 1;
    setsockopt(sock, SOL_SOCKET, SO_BROADCAST, &on, sizeof(on));
    return sock;
}

static struct sockaddr_in broadcast_address(esp_netif_t *netif)
{
    esp_netif_ip_info_t ip;
    ESP_ERROR_CHECK(esp_netif_get_ip_info(netif, &ip));
    struct sockaddr_in to = {
        .sin_family = AF_INET,
        .sin_port = htons(PORT),
        .sin_addr.s_addr = ip.ip.addr | ~ip.netmask.addr,
    };
    return to;
}

void app_main(void)
{
    ESP_ERROR_CHECK(nvs_flash_init());
    esp_netif_t *netif = start_ap();
    int sock = open_broadcast_socket();
    struct sockaddr_in to = broadcast_address(netif);
    ESP_LOGI(TAG, "hidden AP %s up, broadcasting to %s:%d", SSID, inet_ntoa(to.sin_addr), PORT);

    for (;;) {
        if (sendto(sock, MESSAGE, strlen(MESSAGE), 0, (struct sockaddr *)&to, sizeof(to)) < 0)
            ESP_LOGW(TAG, "sendto failed");
        vTaskDelay(pdMS_TO_TICKS(SEND_EVERY_MS));
    }
}
